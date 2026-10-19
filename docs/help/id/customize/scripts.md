# Skrip dan otomatisasi

Skrip mengotomatiskan pekerjaan berulang: menggambar bentuk dari data, mengubah
banyak objek sekaligus, atau menambahkan format berkas. Skrip ditulis dalam
JavaScript di panel [[docker.scripts]] ([[menu.tools > menu.tools.scripts >
menu.tools.scripts_docker]]).

![Panel Skrip](../../img/customize-scripts.png)

## Menulis dan menjalankan skrip {#run}

Skrip bekerja dengan objek-objek TraceDraw: `Application`,
`ActiveDocument`, `ActivePage`, `ActiveLayer`, serta bentuk, isian, garis
luar, warna, kurva, dan efek. Contohnya:

```js
for (let i = 0; i < 5; i++) {
  ActiveLayer.CreateRectangle(10 + i * 25, 10, 20, 20);
}
```

- [[docker.run]] menjalankan skrip. Perubahannya menjadi satu langkah
  urungkan.
- [[docker.record]] merekam apa yang Anda lakukan sebagai skrip (juga
  [[menu.tools > menu.tools.scripts > menu.tools.scripts_record]]).
- [[docker.load]] dan [[docker.save]] membuka dan menyimpan skrip;
  [[menu.tools.scripts_run]] menjalankan berkas skrip.
- [[docker.clear]] mengosongkan penyunting; [[docker.output]] menampilkan apa
  yang dicetak skrip dan galatnya.

## Debugging {#debug}

Di desktop, klik di margin penyunting untuk memasang titik henti, lalu klik
[[docker.debug]]. Skrip berhenti di titik henti: **Lanjutkan**, **Langkah**,
atau **Hentikan**, dan ketik ekspresi untuk mengevaluasinya di tempat skrip
dijeda.

## Filter skrip {#filters}

Skrip yang mendefinisikan `TraceDrawFilter`, dengan nama, ekstensi, dan fungsi
`Import` atau `Export`, menambahkan format berkas ke [[menu.file.import]] dan
[[menu.file.export]]. Klik [[docker.install_filter]] untuk menyimpannya;
[[docker.script_filters]] mencantumkan filter yang terpasang dengan
[[docker.edit]] dan [[docker.remove]].

## Asisten AI (MCP) {#mcp}

TraceDraw mendukung Model Context Protocol, sehingga asisten AI di komputer
Anda dapat membuka, menyunting, memeriksa, mengekspor, dan menyimpan gambar:

- `tracedraw --mcp` menjalankan TraceDraw tanpa jendela untuk asisten.
- Agar asisten dapat menyunting gambar yang terbuka di jendela Anda, aktifkan
  [[options.mcp_window]] di halaman [[options.general]] pada
  [[menu.tools > menu.tools.options > menu.tools.options_global]], dan berikan
  `tracedraw --mcp-window` sebagai perintah asisten. Setiap perubahan adalah
  satu langkah urungkan yang dapat Anda batalkan. Hanya program di komputer
  Anda yang memiliki token rahasia yang ditulis TraceDraw yang dapat
  terhubung.

## Alat baris perintah {#cli}

Setiap unduhan juga menyertakan `tracedraw-cli`, yang mengonversi gambar antar
format tanpa membuka program, untuk pekerjaan massal.
