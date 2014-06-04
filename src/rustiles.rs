#![feature(phase)]

extern crate sync;
extern crate http;
extern crate test;
extern crate gdal;

#[phase(syntax)]
extern crate regex_macros;
extern crate regex;

use std::vec::Vec;
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::io::Writer;
use std::task;
use http::server::{Config, Server, Request, ResponseWriter};
use http::server::request::AbsolutePath;
use http::status::NotFound;
use http::headers;
use tile::raster_tile_worker;
use workqueue::{WorkQueue, WorkQueueProxy};

mod tile;
mod workqueue;



#[test]
fn test_nothing() {
    assert_eq!(1, 1);
}


#[deriving(Clone)]
struct TileServer {
    raster_queue: WorkQueueProxy<(int, int, int), Vec<u8>>,
}


impl TileServer {
    fn handle_raster_tile(&self, x: int, y: int, z: int, w: &mut ResponseWriter) {
        w.headers.content_type = Some(headers::content_type::MediaType {
            type_: "image".to_string(),
            subtype: "png".to_string(),
            parameters: Vec::new(),
        });
        let tile_png = self.raster_queue.push((x, y, z)).recv();
        w.write(tile_png.as_slice()).unwrap();
    }

    fn handle_static(&self, filename: &str, w: &mut ResponseWriter) {
        w.headers.content_type = Some(headers::content_type::MediaType {
            type_: "text".to_string(),
            subtype: "html".to_string(),
            parameters: Vec::new(),
        });

        let content = match filename {
            "/" => index_html.as_bytes(),
            "/raster" => raster_html.as_bytes(),
            "/vector" => vector_html.as_bytes(),
            _   => { self.handle_404(w); return }
        };
        w.write(content).unwrap();
    }

    fn handle_404(&self, w: &mut ResponseWriter) {
        w.status = NotFound;
        w.write("Page not found :(\n".as_bytes()).unwrap();
    }

    fn handle(&self, r: &Request, w: &mut ResponseWriter) {
        w.headers.content_type = Some(headers::content_type::MediaType {
            type_: "text".to_string(),
            subtype: "html".to_string(),
            parameters: Vec::new(),
        });

        match r.request_uri {
            AbsolutePath(ref url) => {
                match regex!(r"^/raster/(\d+)/(\d+)/(\d+)").captures(url.as_slice()) {
                    Some(caps) => {
                        self.handle_raster_tile(
                            from_str::<int>(caps.at(2)).unwrap(),
                            from_str::<int>(caps.at(3)).unwrap(),
                            from_str::<int>(caps.at(1)).unwrap(),
                            w);
                        return
                    },
                    None => {}
                };

                self.handle_static(url.as_slice(), w);
            },
            _ => self.handle_404(w)
        };
    }
}


impl Server for TileServer {
    fn get_config(&self) -> Config {
        Config {
            bind_address: SocketAddr {
                ip: Ipv4Addr(0, 0, 0, 0),
                port: 8001,
            },
        }
    }

    fn handle_request(&self, r: &Request, w: &mut ResponseWriter) {
        self.handle(r, w);
    }
}


fn main() {
    use std::os::args;
    let source_path = Path::new(args().get(1).as_slice());
    let (raster_queue, dispatcher) = WorkQueue::<(int, int, int), Vec<u8>>();
    task::spawn(proc() { dispatcher.run(); });
    for _ in range(0, 4) {
        raster_tile_worker(&raster_queue, &source_path);
    }
    TileServer{raster_queue: raster_queue.proxy()}.serve_forever();
}


static index_html: &'static str = "<!doctype html>\n\
<meta charset='utf-8'>\n\
<p><a href=/raster>raster</a>
<p><a href=/vector>vector</a>
\n\
";


static raster_html: &'static str = "<!doctype html>\n\
<meta charset='utf-8'>\n\
<title>RusTiles raster demo</title>\n\
<link rel='stylesheet' href='//cdnjs.cloudflare.com/ajax/libs/leaflet/0.7.2/leaflet.css'>\n\
<style>
html, body, #map { margin: 0; height: 100%; }
#slider { position: fixed; top: 0; right: 0; }
</style>
<div id='map'></div>
<div id='slider'><input type='range' min='0' max='100' value='50'></div>
<script src='//cdnjs.cloudflare.com/ajax/libs/jquery/2.1.0/jquery.min.js'></script>\n\
<script src='//cdnjs.cloudflare.com/ajax/libs/leaflet/0.7.2/leaflet.js'></script>\n\
<script>
var map = L.map('map').setView([40, 10], 3);
L.tileLayer('http://{s}.tile.osm.org/{z}/{x}/{y}.png', {
  attribution: '&copy; <a href=\\'http://osm.org/copyright\\'>' +
               'OpenStreetMap</a> contributors'}).addTo(map);
var nasa = L.tileLayer('/raster/{z}/{x}/{y}').addTo(map);
function updateOpacity() { nasa.setOpacity(+($('input').val()) / 100); }
$('input').change(updateOpacity); updateOpacity();
</script>
";


static vector_html: &'static str = "<!doctype html>\n\
<meta charset='utf-8'>\n\
<title>RusTiles vector demo</title>\n\
<style>\n\
body { margin: 0; }\n\
.map { position: relative; overflow: hidden; }\n\
.layer { position: absolute; }\n\
.tile { position: absolute; width: 256px; height: 256px; }\n\
.tile path { fill: none; stroke: #000; stroke-linejoin: round; stroke-linecap: round; }\n\
.tile .major_road { stroke: #776; }\n\
.tile .minor_road { stroke: #ccb; }\n\
.tile .highway { stroke: #f39; stroke-width: 1.5px; }\n\
.tile .rail { stroke: #7de; }\n\
.info { position: absolute; bottom: 10px; left: 10px; }\n\
</style>\n\
<body>\n\
<script src='http://d3js.org/d3.v3.min.js'></script>\n\
<script src='http://d3js.org/d3.geo.tile.v0.min.js'></script>\n\
<script>\n\
var width = Math.max(960, window.innerWidth),\n\
    height = Math.max(500, window.innerHeight),\n\
    prefix = prefixMatch(['webkit', 'ms', 'Moz', 'O']);\n\
var tile = d3.geo.tile()\n\
    .size([width, height]);\n\
var projection = d3.geo.mercator()\n\
    .scale((1 << 21) / 2 / Math.PI)\n\
    .translate([-width / 2, -height / 2]); // just temporary\n\
var tileProjection = d3.geo.mercator();\n\
var tilePath = d3.geo.path()\n\
    .projection(tileProjection);\n\
var zoom = d3.behavior.zoom()\n\
    .scale(projection.scale() * 2 * Math.PI)\n\
    .scaleExtent([1 << 20, 1 << 23])\n\
    .translate(projection([-74.0064, 40.7142]).map(function(x) { return -x; }))\n\
    .on('zoom', zoomed);\n\
var map = d3.select('body').append('div')\n\
    .attr('class', 'map')\n\
    .style('width', width + 'px')\n\
    .style('height', height + 'px')\n\
    .call(zoom)\n\
    .on('mousemove', mousemoved);\n\
var layer = map.append('div')\n\
    .attr('class', 'layer');\n\
var info = map.append('div')\n\
    .attr('class', 'info');\n\
zoomed();\n\
function zoomed() {\n\
  var tiles = tile\n\
      .scale(zoom.scale())\n\
      .translate(zoom.translate())\n\
      ();\n\
  projection\n\
      .scale(zoom.scale() / 2 / Math.PI)\n\
      .translate(zoom.translate());\n\
  var image = layer\n\
      .style(prefix + 'transform', matrix3d(tiles.scale, tiles.translate))\n\
    .selectAll('.tile')\n\
      .data(tiles, function(d) { return d; });\n\
  image.exit()\n\
      .each(function(d) { this._xhr.abort(); })\n\
      .remove();\n\
  image.enter().append('svg')\n\
      .attr('class', 'tile')\n\
      .style('left', function(d) { return d[0] * 256 + 'px'; })\n\
      .style('top', function(d) { return d[1] * 256 + 'px'; })\n\
      .each(function(d) {\n\
        var svg = d3.select(this);\n\
        this._xhr = d3.json('http://' + ['a', 'b', 'c'][(d[0] * 31 + d[1]) % 3] + '.tile.openstreetmap.us/vectiles-highroad/' + d[2] + '/' + d[0] + '/' + d[1] + '.json', function(error, json) {\n\
          var k = Math.pow(2, d[2]) * 256; // size of the world in pixels\n\
          tilePath.projection()\n\
              .translate([k / 2 - d[0] * 256, k / 2 - d[1] * 256]) // [0°,0°] in pixels\n\
              .scale(k / 2 / Math.PI);\n\
          svg.selectAll('path')\n\
              .data(json.features.sort(function(a, b) { return a.properties.sort_key - b.properties.sort_key; }))\n\
            .enter().append('path')\n\
              .attr('class', function(d) { return d.properties.kind; })\n\
              .attr('d', tilePath);\n\
        });\n\
      });\n\
}\n\
function mousemoved() {\n\
  info.text(formatLocation(projection.invert(d3.mouse(this)), zoom.scale()));\n\
}\n\
function matrix3d(scale, translate) {\n\
  var k = scale / 256, r = scale % 1 ? Number : Math.round;\n\
  return 'matrix3d(' + [k, 0, 0, 0, 0, k, 0, 0, 0, 0, k, 0, r(translate[0] * scale), r(translate[1] * scale), 0, 1 ] + ')';\n\
}\n\
function prefixMatch(p) {\n\
  var i = -1, n = p.length, s = document.body.style;\n\
  while (++i < n) if (p[i] + 'Transform' in s) return '-' + p[i].toLowerCase() + '-';\n\
  return '';\n\
}\n\
function formatLocation(p, k) {\n\
  var format = d3.format('.' + Math.floor(Math.log(k) / 2 - 2) + 'f');\n\
  return (p[1] < 0 ? format(-p[1]) + '°S' : format(p[1]) + '°N') + ' '\n\
       + (p[0] < 0 ? format(-p[0]) + '°W' : format(p[0]) + '°E');\n\
}\n\
</script>\n\
";
