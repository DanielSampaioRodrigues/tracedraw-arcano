//! Hardware acceleration (Options > General, #445): the window is drawn
//! with wgpu on the graphics card chosen there (the fastest one when none
//! is), or with the system's software adapter when it is off. A start
//! whose renderer cannot open the window starts TraceDraw again with the
//! next one, hardware, then OpenGL, then software, so a virtual machine
//! without a usable graphics card still opens (#439), and a computer
//! whose card only OpenGL can drive is not drawn on the CPU (#616). See
//! docs/behavior/about-and-startup.md.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

/// How a start draws the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// wgpu on a graphics card, never on a software adapter (#616).
    Hardware,
    /// wgpu on the software adapter (WARP on Windows, a CPU Vulkan
    /// driver elsewhere).
    Software,
    /// OpenGL, the renderer of earlier versions.
    Gl,
}

/// The command-line switch that names the renderer of a start.
const ARG: &str = "--renderer=";

impl Mode {
    pub fn arg(self) -> String {
        format!("{ARG}{}", self.name())
    }

    /// The renderer `--renderer=` names among `args`.
    pub fn from_args(args: &[String]) -> Option<Mode> {
        args.iter()
            .find_map(|a| a.strip_prefix(ARG).and_then(Mode::from_name))
    }

    /// How the settings remember it.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Hardware => "hardware",
            Mode::Software => "software",
            Mode::Gl => "gl",
        }
    }

    fn from_name(name: &str) -> Option<Mode> {
        [Mode::Hardware, Mode::Software, Mode::Gl]
            .into_iter()
            .find(|m| m.name() == name)
    }

    /// The renderer a start uses: the one `--renderer=` names (a start
    /// before could not open the window), else the one that opened it
    /// last time when the first choice did not, else the graphics card or,
    /// with hardware acceleration off, the software adapter.
    pub fn for_start(args: &[String], s: &crate::settings::Settings) -> Mode {
        let first = Mode::first(s);
        Mode::from_args(args)
            .or_else(|| Mode::from_name(&s.working_renderer).filter(|m| *m != first))
            .unwrap_or(first)
    }

    /// The renderer Options asks for: the graphics card, or the software
    /// adapter with hardware acceleration off.
    pub fn first(s: &crate::settings::Settings) -> Mode {
        if s.hardware_acceleration {
            Mode::Hardware
        } else {
            Mode::Software
        }
    }

    /// The renderers tried in turn when `first` is the choice: OpenGL
    /// before the software adapter, which draws every frame on the CPU,
    /// and the software adapter last because it always opens (#616).
    pub fn chain(first: Mode) -> &'static [Mode] {
        match first {
            Mode::Hardware => &[Mode::Hardware, Mode::Gl, Mode::Software],
            Mode::Software => &[Mode::Software, Mode::Gl],
            Mode::Gl => &[Mode::Gl, Mode::Software],
        }
    }

    /// The renderer to try when this one cannot open the window.
    pub fn next(self, first: Mode) -> Option<Mode> {
        let chain = Mode::chain(first);
        let at = chain.iter().position(|m| *m == self)?;
        chain.get(at + 1).copied()
    }
}

/// What the adapter choice knows of one adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub name: String,
    pub kind: eframe::wgpu::DeviceType,
    /// Whether it can draw on the window.
    pub surface: bool,
}

/// The adapter a start in `mode` draws with (an index into `list`), and
/// for each adapter why it was taken or left, for the log. `wanted` names
/// the graphics card chosen in Options (empty: the fastest). Hardware
/// mode never takes a software adapter: the start fails instead and the
/// next renderer is tried (#616).
pub fn choose(mode: Mode, wanted: &str, list: &[Candidate]) -> (Option<usize>, Vec<&'static str>) {
    let software = |c: &Candidate| c.kind == eframe::wgpu::DeviceType::Cpu;
    let fits = |c: &Candidate| {
        c.surface
            && match mode {
                Mode::Software => software(c),
                _ => !software(c),
            }
    };
    let named = (!wanted.is_empty())
        .then(|| list.iter().position(|c| fits(c) && c.name == wanted))
        .flatten();
    let pick = named.or_else(|| {
        list.iter()
            .enumerate()
            .filter(|(_, c)| fits(c))
            .min_by_key(|(_, c)| rank(c.kind))
            .map(|(i, _)| i)
    });
    let notes = list
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if !c.surface {
                "rejected: cannot draw on the window"
            } else if mode == Mode::Software && !software(c) {
                "rejected: not a software adapter (hardware acceleration is off)"
            } else if mode != Mode::Software && software(c) {
                "rejected: software adapter (hardware acceleration wants a graphics card)"
            } else if Some(i) == pick && named.is_some() {
                "picked: the card chosen in Options"
            } else if Some(i) == pick {
                "picked: the fastest usable"
            } else {
                "usable, not picked"
            }
        })
        .collect();
    (pick, notes)
}

static IN_USE: OnceLock<String> = OnceLock::new();

/// Whether the window of this run opened (the app is being built): a
/// panic before that is a renderer that failed to start, after it one of
/// the app.
static OPENED: AtomicBool = AtomicBool::new(false);

pub fn window_opened() -> bool {
    OPENED.load(Ordering::SeqCst)
}

/// What draws the window in this run, as Options shows it.
pub fn in_use() -> Option<&'static str> {
    IN_USE.get().map(String::as_str)
}

/// Note what the window was opened with; a renderer other than the
/// first choice is remembered, so the next start goes straight to it.
pub fn note_renderer(cc: &eframe::CreationContext<'_>, mode: Mode) {
    OPENED.store(true, Ordering::SeqCst);
    crate::startup_log::quiet_panics(false);
    let mut s = crate::settings::Settings::load();
    let first = Mode::first(&s);
    let remember = if mode == first { "" } else { mode.name() };
    if s.working_renderer != remember {
        s.working_renderer = remember.to_string();
        s.save();
    }
    let what = match cc.wgpu_render_state.as_ref() {
        Some(rs) => {
            let i = rs.adapter.get_info();
            format!("{} ({:?})", i.name, i.backend)
        }
        None => "OpenGL".to_string(),
    };
    if let Some(gl) = cc.gl.as_ref() {
        use eframe::glow::HasContext;
        // SAFETY: plain queries on the current context eframe just made.
        let [vendor, renderer, version, glsl] = [
            eframe::glow::VENDOR,
            eframe::glow::RENDERER,
            eframe::glow::VERSION,
            eframe::glow::SHADING_LANGUAGE_VERSION,
        ]
        .map(|p| unsafe { gl.get_parameter_string(p) });
        log::info!("OpenGL: {vendor}, {renderer}, version {version}, GLSL {glsl}");
    }
    log::info!("the window opened with {mode:?}: {what}");
    let _ = IN_USE.set(what);
}

/// The graphics adapters of this computer by name, software ones last;
/// looked for once.
pub fn adapters() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| {
        if cfg!(test) {
            return Vec::new();
        }
        let instance = eframe::wgpu::Instance::new(
            eframe::wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
        );
        let found =
            pollster::block_on(instance.enumerate_adapters(eframe::wgpu::Backends::PRIMARY));
        let mut list: Vec<(u8, String)> = found
            .iter()
            .map(|a| {
                let i = a.get_info();
                (rank(i.device_type), i.name)
            })
            .collect();
        list.sort();
        let mut names: Vec<String> = Vec::new();
        for (_, n) in list {
            if !names.contains(&n) {
                names.push(n);
            }
        }
        names
    })
}

/// The order adapters are preferred in: separate cards, built-in ones,
/// virtual ones, others, then software.
fn rank(t: eframe::wgpu::DeviceType) -> u8 {
    use eframe::wgpu::DeviceType;
    match t {
        DeviceType::DiscreteGpu => 0,
        DeviceType::IntegratedGpu => 1,
        DeviceType::VirtualGpu => 2,
        DeviceType::Other => 3,
        DeviceType::Cpu => 4,
    }
}

/// The renderer and the wgpu set-up of a start in `mode`; `wanted` names
/// the graphics card chosen (empty: the fastest).
pub fn options(
    mode: Mode,
    wanted: &str,
) -> (eframe::Renderer, eframe::egui_wgpu::WgpuConfiguration) {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    if mode == Mode::Gl {
        return (eframe::Renderer::Glow, WgpuConfiguration::default());
    }
    let wanted = wanted.to_string();
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.native_adapter_selector = Some(Arc::new(move |list, surface| {
        let candidates: Vec<Candidate> = list
            .iter()
            .map(|a| {
                let i = a.get_info();
                Candidate {
                    name: i.name,
                    kind: i.device_type,
                    surface: surface.is_none_or(|s| a.is_surface_supported(s)),
                }
            })
            .collect();
        let (pick, notes) = choose(mode, &wanted, &candidates);
        log::info!("{} graphics adapter(s) for {mode:?}:", list.len());
        for (a, note) in list.iter().zip(&notes) {
            let i = a.get_info();
            log::info!(
                "  {} ({:?}, {:?}, {:04x}:{:04x}, driver {} {}): {note}",
                i.name,
                i.backend,
                i.device_type,
                i.vendor,
                i.device,
                i.driver,
                i.driver_info
            );
        }
        match pick.and_then(|i| list.get(i)) {
            Some(a) => Ok(a.clone()),
            None if mode == Mode::Software => {
                Err("no software adapter can draw the window".to_string())
            }
            None => Err(
                "no graphics card can draw the window (a software adapter is left to the last renderer)"
                    .to_string(),
            ),
        }
    }));
    let config = WgpuConfiguration {
        // One frame queued, not two: what is drawn under the pointer (a
        // shape being drawn, a drawn pointer) trails it by a frame less
        // (#655).
        surface: eframe::egui_wgpu::SurfaceConfig::LOW_LATENCY,
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    };
    (eframe::Renderer::Wgpu, config)
}

/// Run TraceDraw again with `next`, the other arguments kept, until it
/// ends; false when it did not start.
pub fn restart_with(next: Mode, args: &[String]) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let mut rest: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with(ARG))
        .cloned()
        .collect();
    rest.push(next.arg());
    std::process::Command::new(exe).args(rest).status().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_renderer_that_fails_hands_over_to_the_next() {
        let args = vec!["drawing.cdr".to_string()];
        assert_eq!(Mode::from_args(&args), None);
        let mut m = Mode::Hardware;
        let mut seen = vec![m];
        while let Some(n) = m.next(Mode::Hardware) {
            let mut with = args.clone();
            with.push(n.arg());
            assert_eq!(Mode::from_args(&with), Some(n));
            seen.push(n);
            m = n;
        }
        // OpenGL before the software adapter, which draws on the CPU and
        // always opens, so it comes last (#616).
        assert_eq!(seen, vec![Mode::Hardware, Mode::Gl, Mode::Software]);
        // Hardware acceleration off: the software adapter, then OpenGL.
        assert_eq!(Mode::Software.next(Mode::Software), Some(Mode::Gl));
        assert_eq!(Mode::Gl.next(Mode::Software), None);
        // Remembered OpenGL that fails falls to the software adapter.
        assert_eq!(Mode::Gl.next(Mode::Gl), Some(Mode::Software));
        assert_eq!(Mode::Software.next(Mode::Gl), None);
        // The renderer that opened the window last time, unless it is the
        // first choice anyway; hardware acceleration off starts in software.
        let mut s = crate::settings::Settings::default();
        assert_eq!(Mode::for_start(&args, &s), Mode::Hardware);
        s.working_renderer = "gl".into();
        assert_eq!(Mode::for_start(&args, &s), Mode::Gl);
        s.working_renderer.clear();
        s.hardware_acceleration = false;
        assert_eq!(Mode::for_start(&args, &s), Mode::Software);
        assert_eq!(Mode::for_start(&[Mode::Gl.arg()], &s), Mode::Gl);
        assert_eq!(options(Mode::Gl, "").0, eframe::Renderer::Glow);
        assert_eq!(options(Mode::Hardware, "").0, eframe::Renderer::Wgpu);
    }

    fn adapter(name: &str, kind: eframe::wgpu::DeviceType, surface: bool) -> Candidate {
        Candidate {
            name: name.into(),
            kind,
            surface,
        }
    }

    /// Hardware mode takes the card named in Options, else the fastest
    /// card, and never the software adapter, which only a start with
    /// hardware acceleration off (or the last renderer) uses (#616).
    #[test]
    fn hardware_mode_never_draws_on_the_software_adapter() {
        use eframe::wgpu::DeviceType::*;
        let warp = adapter("Microsoft Basic Render Driver", Cpu, true);
        // Only the software adapter: hardware mode fails, so the start
        // goes on to OpenGL.
        let only = [warp.clone()];
        let (pick, notes) = choose(Mode::Hardware, "", &only);
        assert_eq!(pick, None);
        assert!(notes[0].starts_with("rejected: software adapter"));
        assert_eq!(choose(Mode::Software, "", &only).0, Some(0));
        let list = [
            warp.clone(),
            adapter("Built-in", IntegratedGpu, true),
            adapter("Card", DiscreteGpu, true),
            adapter("Elsewhere", DiscreteGpu, false),
        ];
        let (pick, notes) = choose(Mode::Hardware, "", &list);
        assert_eq!(pick, Some(2));
        assert_eq!(notes[2], "picked: the fastest usable");
        assert_eq!(notes[1], "usable, not picked");
        assert_eq!(notes[3], "rejected: cannot draw on the window");
        let (pick, notes) = choose(Mode::Hardware, "Built-in", &list);
        assert_eq!(pick, Some(1));
        assert_eq!(notes[1], "picked: the card chosen in Options");
        // A chosen card that is gone or cannot draw: the fastest.
        assert_eq!(choose(Mode::Hardware, "Elsewhere", &list).0, Some(2));
        // Naming the software adapter does not let hardware mode use it.
        let named = choose(Mode::Hardware, "Microsoft Basic Render Driver", &only);
        assert_eq!(named.0, None);
        let (pick, notes) = choose(Mode::Software, "", &list);
        assert_eq!(pick, Some(0));
        assert!(notes[2].starts_with("rejected: not a software adapter"));
        assert_eq!(choose(Mode::Hardware, "", &[]).0, None);
    }
}
