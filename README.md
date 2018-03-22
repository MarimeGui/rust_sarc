# SARC in Rust

Yet another SARC extractor, but this time in Rust !

Thanks to:
* [Custom Mario Kart 8 Wiki](http://mk8.tockdom.com/wiki/SARC_(File_Format))
* [SARCTools](https://github.com/NWPlayer123/WiiUTools/tree/master/SARCTools)

To Extract a (compressed or not) SARC file:
```
$ cargo run --release --bin extract your_file.szs output_folder
```