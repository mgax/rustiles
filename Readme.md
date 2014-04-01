## RusTiles ##

An experimental XYZ tile server written in the
[Rust](http://www.rust-lang.org/) programming language.

How to run it:

* Download and build
  [rust-http](https://github.com/chris-morgan/rust-http) and
  [rust-gdal](https://github.com/mgax/rust-gdal)

* Download a [NASA Blue Marble](http://visibleearth.nasa.gov/view.php?id=73909)
  image, or any other raster image in
  [equirectangular projection](http://en.wikipedia.org/wiki/Equirectangular_projection)
* Build the code:

  ```
  RUSTFLAGS='-L ../rust-http/build -L ../rust-gdal/build' make build/rustiles
  ```

* Run the server:

  ```
  build/rustiles world.topo.bathy.200412.3x21600x10800.jpg
  ```

* Connect to http://localhost:8001/ and enjoy the view :)
