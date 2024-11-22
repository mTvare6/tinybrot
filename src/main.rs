use num::complex::Complex;
use rayon::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

mod colormaps;
use crate::colormaps::*;

const R:f64 = 1.;

fn pixel_coordinates(px: usize, py: usize, config: &Config) -> (f64, f64) {
    let x =
        ((config.xu - config.xl) * (px as f64 / config.sw as f64) + config.xl) * config.aspect_ratio;
    let y = (config.yu - config.yl) * (py as f64 / config.sh as f64) + config.yl;
    return (x, y);
}

struct Config {
    map: ColorMap,
    w: usize,
    h: usize,
    xl: f64,
    xu: f64,
    yl: f64,
    yu: f64,
    #[allow(unused)]
    supersampling: usize,
    outfile: String,
    aspect_ratio: f64,
    sw: usize,
    sh: usize,
}

fn parse_args() -> Result<Config, lexopt::Error> {
    use lexopt::prelude::*;

    let maps = get_all_color_maps();

    let mut w = 4096;
    let mut h = 4096;
    let mut scheme = None;

    let mut outfile = None;
    let mut supersampling = 1;
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('W') | Long("width") => {
                w = parser.value()?.parse()?;
            }
            Short('H') | Long("height") => {
                h = parser.value()?.parse()?;
            }
            Short('s') | Long("supersample") => {
                supersampling = parser.value()?.parse()?;
            }
            Short('c') | Long("color") if scheme.is_none() => {
                scheme = Some(parser.value()?.string()?);
            }
            Value(val) if outfile.is_none() => {
                outfile = Some(val.string()?);
            }
            Short('h') | Long("help") => {
                println!("Usage: tinybrot [-W|--width=NUM] [-H|--height=NUM] [-s|--supersample=NUM] [-c|--color=SCHEME] out.png");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    let map = if let Some(name) = scheme {
        maps.get(&name).unwrap_or(&maps["inferno"]).clone()
    } else {
        maps["inferno"].clone()
    };

    Ok(Config {
        map,
        w,
        h,
        xl: -R,
        xu: R,
        yl: -R,
        yu: R,
        supersampling,
        aspect_ratio: w as f64 / h as f64,
        outfile: outfile.ok_or("missing argument output file")?,
        sw: supersampling * w,
        sh: supersampling * h,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_args()?;
    let path = Path::new(&config.outfile);
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, config.w as u32, config.h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));
    let source_chromaticities = png::SourceChromaticities::new(
        (0.31270, 0.32900),
        (0.64000, 0.33000),
        (0.30000, 0.60000),
        (0.15000, 0.06000),
    );
    encoder.set_source_chromaticities(source_chromaticities);
    let mut writer = encoder.write_header().unwrap();

    let mut data = vec![0; config.sw * config.sh * 4];

    let max_iteration = 10000;
    let c = Complex{re: 0.355534, im:-0.337292};
    
    data.par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
        let py = i / config.sw;
        let px = i - py * config.sw;
        let (re, im) = pixel_coordinates(px, py, &config);
        let mut z = Complex{re, im};

        let mut iteration = 0;
        while z.norm_sqr() <= 9. && iteration < max_iteration {
            z = z.powu(2) + c;
            iteration += 1;
        }
        let c = iteration_color(iteration, &config.map);
        chunk.copy_from_slice(&c);
    });
    writer.write_image_data(&data).unwrap();
    Ok(())
}
