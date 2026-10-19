# Menjiplak bitmap

Menjiplak mengubah bitmap menjadi bentuk vektor yang dapat Anda sunting, warnai
ulang, dan skalakan tanpa kehilangan kualitas: logo hasil pindai, sketsa,
foto.

## Jiplak cepat {#quick-trace}

Pilih bitmap lalu pilih [[menu.bitmaps > menu.bitmaps.quick_trace]]. Perintah
ini memakai preset yang dipilih di halaman [[options.bitmap_tracing]] pada
opsi global (awalnya Clipart).

## Kotak dialog Jiplak Bitmap {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] dan
[[menu.bitmaps.centerline_trace]] berisi daftar preset; pilih salah satu untuk
membuka kotak dialog jiplak.

- **Jiplak garis luar** mengisi area warna dengan bentuk:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]], dan [[menu.bitmaps.trace_high_quality]].
- **Jiplak garis tengah** mengikuti garis dengan kurva terbuka:
  [[menu.bitmaps.trace_technical]] dan [[menu.bitmaps.trace_line_drawing]].

![Kotak dialog Jiplak Bitmap](../../img/bitmaps-tracing.png)

Kotak dialog menampilkan bitmap **sebelum** dan hasil jiplaknya **sesudah**,
diperbarui saat Anda mengubah pengaturan:

- tombol preset di bagian atas;
- **Detail** (0 sampai 100): detail lebih banyak mempertahankan area yang lebih
  kecil;
- **Penghalusan** (0 sampai 100, awalnya 25): kurva lebih halus, simpul lebih
  sedikit;
- **Kehalusan sudut** (0 sampai 100): sudut lebih bulat;
- **Hitam dan putih**, atau jumlah **Warna** (1 sampai 256);
- **Hapus latar belakang** (awalnya aktif): mengabaikan warna latar belakang;
- **Hapus gambar asli** (awalnya nonaktif);
- **Pratinjau** menghitung kurva dari jiplakan penuh.

Klik [[dialog.ok]] untuk menjiplak. Bentuk-bentuk diletakkan di atas bitmap,
yang paling gelap lebih dahulu.
