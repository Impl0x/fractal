extern crate num;
extern crate bmp;

mod gradient;
mod render;
mod fractal;

use bmp::{Image, Pixel};
use num::complex::Complex;
use gradient::{Gradient, Stop};
use render::{Camera};
use fractal::*;

use std::thread;
use std::sync::Arc;
use std::sync::mpsc::channel;

const SCALE:usize = 1;
const WIDTH:usize  = 2880 * SCALE;
const HEIGHT:usize = 1800 * SCALE; 

const MAX_ITERS:u32 = 1000;

fn pix(r: u8, g: u8, b: u8) -> Pixel {
    Pixel { r: r, g: g, b: b }
}

#[inline]
fn get_row_points(origin: Complex<f64>, p_size: f64, col: usize) -> Vec<Complex<f64>> {
    let mut points = Vec::with_capacity(WIDTH as usize);
    let i = origin.im + p_size * (col as f64);
    for x in 0..WIDTH {
        let r = origin.re + p_size * (x as f64);
        points.push(Complex::new(r, i));
    }
    return points;
}

fn make_plot<F>(cam: &Camera, eval: Arc<F>) -> Vec<Vec<f32>> 
where F: 'static + Send + Sync + Fn(Complex<f64>, u32) -> f32 {
    let n_threads = 2;

    let (origin, p_size) = cam.find_origin_and_pixel_size(WIDTH as u32, HEIGHT as u32);
    let (agg_chan_in, agg_chan_out) = channel();

    let mut threads = Vec::new();
    for thread in 0..n_threads {
        let agg_chan_in = agg_chan_in.clone();
        let eval = eval.clone();

        threads.push(thread::spawn(move || {
            println!("Thread {} starting.", thread);
            for data_idx in 0..(HEIGHT / n_threads) {
                let y = (data_idx * n_threads) + thread;
                let mut results = Vec::with_capacity(WIDTH as usize);
                let ref row = get_row_points(origin, p_size, y);
                for x in 0..row.len() {
                    results.push((x as u32, y, eval(row[x], MAX_ITERS)));
                }
                agg_chan_in.send(results).unwrap();
            }
            println!("Thread {} ending.", thread);
        }));
    }

    let mut plot = (0..WIDTH).map(|_| {
        (0..HEIGHT).map(|_| 0.0).collect::<Vec<f32>>()
    }).collect::<Vec<Vec<f32>>>();

    println!("Starting to receive thread output");
    for _ in 0..HEIGHT {
        let result = agg_chan_out.recv().unwrap();
        for (x, y, iters) in result {
            plot[x as usize][y as usize] = iters.into();
        }
    }

    println!("Checking all threads have ended.");
    for t in threads {
        t.join().unwrap();
    }
    println!("Finished generating plot!");

    return plot;
}

fn calc_hist(plot: &Vec<Vec<u32>>) -> Vec<u32> {
    let mut hist: Vec<u32> = (0..MAX_ITERS+1).map(|_| 0).collect();
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            hist[plot[x][y] as usize] += 1;
        }
    }
    return hist;
}

fn make_image(plot: &Vec<Vec<f32>>, hist: &[u32], grad: Gradient) -> Image {
    /*let mut total = 0.0;
    for i in 0..MAX_ITERS {
        total += hist[i as usize] as f32
    }*/

    let mut img = Image::new(WIDTH as u32, HEIGHT as u32);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            /*let mut hue = 0.0;
            for i in 0..plot[x][y] {
                hue += hist[i as usize] as f32 / total;
            }*/
            let hue = plot[x][y] as f32 / MAX_ITERS as f32;
            let pixel = grad.get_color(hue);
            img.set_pixel(x as u32, y as u32, pixel);
        }
    }
    return img;
}

fn main() {
    let grad = {
        let period = 0.5;
        let initial = pix(0, 0, 0);
        let stops = vec![
            Stop::new(0.05, pix(255,   0,   0)),
            Stop::new(0.2, pix(255, 255,   0)),
            Stop::new(0.3, pix(  0, 255,   0)),
            Stop::new(0.4, pix(  0, 255, 255)),
            Stop::new(0.5, pix(  0,   0, 255)),
            Stop::new(0.6, pix(  0, 255, 255)),
            Stop::new(0.7, pix(  0, 255,   0)),
            Stop::new(0.8, pix(255, 255,   0)),
            Stop::new(0.9, pix(255,   0,   0))];
        let end = pix(0, 0, 0);
        Gradient::new(period, initial, stops, end)
    };
    
    let cam = Camera::new(Complex::new(-0.0, 0.0), -1.0);
    let plot = make_plot(&cam, Arc::new(eval_julia));
    //let hist = &(calc_hist(&plot))[..];
    let img = make_image(&plot, &(vec![])[..], grad);
    let _ = img.save("img-smooth-large.bmp");
}
