use super::material::{Material, MaterialHitResult};
use crate::onb::ONB;
use crate::pdf::{CosinePDF, PDF};
use crate::ray::Ray;
use crate::texture::Sampler2D;
use glam::Vec3;
pub struct Lambert<'a> {
    albedo: &'a dyn Sampler2D,
}

impl<'a> Material for Lambert<'a> {
    fn scatter(
        &self,
        _r_in: &Ray,
        rec: &crate::hittable::HitRecord,
    ) -> Option<super::material::MaterialHitResult> {
        let pdf = Box::new(CosinePDF::new(ONB::new(&rec.normal)));
        return Some(MaterialHitResult {
            color: self.albedo.sample(rec.uv),
            ray: Ray::new(rec.p, pdf.generate()),
            pdf: Some(pdf)
        });
    }

    fn scattering_pdf(&self, r_in: &Ray, hit_record: &crate::hittable::HitRecord, scattered_ray: &Ray) -> f32 {
        let cos_theta = hit_record.normal.normalize().dot(scattered_ray.direction.normalize());
        0.0f32.max(cos_theta / std::f32::consts::PI)
    }
}

impl<'a> Lambert<'a> {
    pub fn new(color: &'a dyn Sampler2D) -> Self {
        Self { albedo: color }
    }
}
