# Updates

Desktop only (the browser build is always the latest; there Help >
Updates opens the releases page).

- Help > Updates asks GitHub for the project's latest release
  (`api.github.com/repos/igtoth/tracedraw/releases/latest`) and says
  whether it is newer than this version (numbers compared as major,
  minor, patch; drafts, pre-releases and the nightly build never count).
- A newer release offers Download (the file for this system: the
  Windows installer, else the portable zip; the macOS disk image for the
  Mac's processor; the Linux AppImage, else the tar.gz; the release page
  when none fits), Release notes, Skip this version and Close. Download
  opens the link; the system's installer does the installing.
- Options > Global > General, Check for updates at start (on by
  default): once a day, the same check runs in the background at start;
  only a newer release that was not skipped is shown, nothing otherwise.
- A failed check (no connection) says why and offers the releases page.

## Checks

- `updates::tests::versions_compare_by_number`
- `updates::tests::the_download_fits_the_system`
- `updates::tests::the_same_or_older_drafts_and_pre_releases_are_not_offered`
- `ui::dialogs::tests::every_dialog_draws_and_accepts_enter` (the dialog)
- Checked against GitHub from the app: 0.4.0 reads as the latest version.
