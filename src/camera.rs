use crate::color::Color;
use crate::hittable::Hittable;
use crate::interval::Interval;
use crate::pdf::{HittablePDF, MixturePDF, PDF};
use crate::rand_vec3::random_vec_unit_disk;
use crate::ray::Ray;
use crate::Float;
use crate::Vec3;
use image::ImageBuffer;
use rand::prelude::*;
use rayon::prelude::*;
#[derive(Default)]
pub struct Camera {
    pub aspect_ratio: Float,
    pub image_width: i32,
    image_height: i32,
    center: Vec3,
    pixel00_loc: Vec3,
    pixel_delta_u: Vec3,
    pixel_delta_v: Vec3,
    num_samples: i32,
    max_depth: i32,
    vertical_fov: Float,
    pub look_from: Vec3,
    pub look_at: Vec3,
    pub up: Vec3,
    u: Vec3,
    v: Vec3,
    w: Vec3,
    defocus_disk_u: Vec3,
    defocus_disk_v: Vec3,
    defocus_angle: Float,
    focus_distance: Float,
    background_color: Color,
}

impl Camera {
    pub fn new(
        aspect: Float,
        width: i32,
        num_samples: i32,
        max_depth: i32,
        look_from: Vec3,
        look_at: Vec3,
        up: Vec3,
        bg: Color,
    ) -> Self {
        let mut cam = Self::default();
        cam.aspect_ratio = aspect;
        cam.image_width = width;
        cam.num_samples = num_samples;
        cam.max_depth = max_depth;
        cam.vertical_fov = 40.0;
        cam.look_from = look_from;
        cam.look_at = look_at;
        cam.up = up;
        cam.defocus_angle = 0.0;
        cam.focus_distance = 3.46;
        cam.background_color = bg;
        cam.initialize();
        cam
    }

    pub fn render(&self, world: &dyn Hittable, important_objs: &dyn Hittable) {
        let input_row: Vec<(i32, i32)> = vec![(0, 0); self.image_width as usize];
        let mut image: Vec<Vec<(i32, i32)>> = vec![input_row.clone(); self.image_height as usize];
        for j in 0..image.len() {
            for i in 0..input_row.len() {
                image[j][i] = (j as i32, i as i32);
            }
        }
        let rendered_image: Vec<Vec<Color>> = image
            .par_iter()
            .map(|row| {
                row.iter()
                    .map(|(j, i)| {
                        let mut color = Color::new(0.0, 0.0, 0.0);
                        for _ in 0..self.num_samples {
                            color += self
                                .ray_color(
                                    &self.get_ray(*i, *j as Float),
                                    world,
                                    important_objs,
                                    self.max_depth,
                                )
                                .clamp();
                        }
                        color.correct_nans();
                        color
                    })
                    .collect()
            })
            .collect();

        let img =
            ImageBuffer::from_fn(self.image_width as u32, self.image_height as u32, |x, y| {
                let c = rendered_image[y as usize][x as usize];
                image::Rgb(c.to_output_array(self.num_samples))
            });
        img.save("image.png").unwrap_or_else(|e| {
            println!("Error: image could not be saved to disk. {}", e.to_string());
        })
    }

    fn initialize(&mut self) {
        self.image_height = (self.image_width as Float / self.aspect_ratio) as i32;
        self.image_height = if self.image_height < 1 {
            1
        } else {
            self.image_height
        };

        self.center = self.look_from;

        // Determine viewport dimensions.
        let theta = self.vertical_fov.to_radians();
        let h = Float::tan(theta / 2.0);
        let viewport_height = 2.0 * h * self.focus_distance;
        let viewport_width =
            viewport_height * (self.image_width as Float / self.image_height as Float);

        self.w = (self.look_from - self.look_at).normalize();
        self.u = self.up.cross(self.w).normalize();
        self.v = self.w.cross(self.u);
        // Calculate the vectors across the horizontal and down the vertical viewport edges.
        let viewport_u = viewport_width * self.u;
        let viewport_v = viewport_height * -self.v;

        // Calculate the horizontal and vertical delta vectors from pixel to pixel.
        self.pixel_delta_u = viewport_u / self.image_width as Float;
        self.pixel_delta_v = viewport_v / self.image_height as Float;

        // Calculate the location of the upper left pixel.
        let viewport_upper_left =
            self.center - (self.focus_distance * self.w) - viewport_u / 2.0 - viewport_v / 2.0;
        self.pixel00_loc = viewport_upper_left + 0.5 * (self.pixel_delta_u + self.pixel_delta_v);

        let defocus_radius =
            self.focus_distance * Float::tan((self.defocus_angle / 2.0).to_radians());
        self.defocus_disk_u = self.u * defocus_radius;
        self.defocus_disk_v = self.v * defocus_radius;
    }

    fn ray_color(
        &self,
        ray: &Ray,
        world: &dyn Hittable,
        important_objs: &dyn Hittable,
        depth: i32,
    ) -> Color {
        if depth <= 0 {
            return Color::new(1.0, 1.0, 1.0);
        }
        if let Some(rec) = world.hit(
            ray,
            Interval {
                min: 0.001,
                max: Float::MAX,
            },
        ) {
            if let Some(mat_hit_res) = rec.material.scatter(ray, &rec) {
                if mat_hit_res.pdf.is_none() {
                    return mat_hit_res.color
                        * self.ray_color(&mat_hit_res.ray, world, important_objs, depth - 1);
                }
                let mat_pdf = mat_hit_res.pdf.unwrap();
                let pdf = HittablePDF::new(rec.p, important_objs);
                let mix_pdf = MixturePDF::new(&pdf, &*mat_pdf);
                let scattered = Ray::new(rec.p, mix_pdf.generate());
                let pdf_value = mix_pdf.value(&scattered.direction);
                let scattering_pdf = rec.material.scattering_pdf(ray, &rec, &scattered);

                // return mat_hit_res.color * self.ray_color(&mat_hit_res.ray, world, important_objs, depth - 1);
                return mat_hit_res.color
                    * self.ray_color(&scattered, world, important_objs, depth - 1)
                    * scattering_pdf
                    / pdf_value;
            } else {
                return rec.material.emit_color(ray, &rec);
            }
        }

        return self.background_color;
    }

    fn get_ray(&self, i: i32, j: Float) -> Ray {
        let pixel_center = self.pixel00_loc
            + (i as Float * self.pixel_delta_u)
            + (j as Float * self.pixel_delta_v);
        let pixel_sample = pixel_center + self.pixel_sample_square();

        let ray_origin = if self.defocus_angle <= 0.0 {
            self.center
        } else {
            self.defocus_disk_sample()
        };
        let ray_direction = pixel_sample - ray_origin;

        return Ray::new(self.center, ray_direction.normalize());
    }

    fn pixel_sample_square(&self) -> Vec3 {
        let px: Float = -0.5 + random::<Float>();
        let py: Float = -0.5 + random::<Float>();

        return (px * self.pixel_delta_u) + (py * self.pixel_delta_v);
    }

    fn defocus_disk_sample(&self) -> Vec3 {
        let p = random_vec_unit_disk();
        return self.center + (p.x * self.defocus_disk_u) + (p.y * self.defocus_disk_v);
    }
}
