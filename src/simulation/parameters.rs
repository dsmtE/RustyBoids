use oxyde::egui;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct InitParametersUniformBufferContent {
    pub seed: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimulationParametersUniformBufferContent {
    pub view_radius: f32,
    //  The separation radius is the view radius times this factor
    pub separation_radius_factor: f32,
    pub cohesion_scale: f32,
    pub aligment_scale: f32,
    pub separation_scale: f32,
    // Grid size is the number of cells per axis
    pub repulsion_margin: f32,
    pub repulsion_strength: f32,
    // to move in an other buffer as they are only used for the grid optimization
    pub boids_count: u32,
    pub grid_size: u32,
}

impl Default for SimulationParametersUniformBufferContent {
    fn default() -> Self {
        let view_radius = 0.02;
        Self {
            view_radius,
            separation_radius_factor: 0.3,
            cohesion_scale: 0.4,
            aligment_scale: 0.9,
            separation_scale: 0.9,
            repulsion_margin: 0.1,
            repulsion_strength: 0.5,
            boids_count: 1024,
            grid_size: grid_size_from_view_radius(view_radius),
        }
    }
}

pub fn grid_size_from_view_radius(view_radius: f32) -> u32 { (1.0 / view_radius).ceil() as u32 }

impl SimulationParametersUniformBufferContent {
    pub fn display_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Simulation settings").default_open(true).show(ui, |ui| {
            ui.add(
                egui::Slider::from_get_set(0.0..=1.0, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.separation_radius_factor = v as f32;
                    }
                    self.separation_radius_factor as f64
                })
                .prefix("separation radius factor"),
            );

            ui.add(
                egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.view_radius = v as f32;
                    }
                    self.view_radius as f64
                })
                .prefix("view radius"),
            );

            ui.add(
                egui::Slider::from_get_set(0.0..=1.0, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.cohesion_scale = v as f32;
                    }
                    self.cohesion_scale as f64
                })
                .prefix("Cohesion scale"),
            );

            ui.add(
                egui::Slider::from_get_set(0.0..=1.0, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.aligment_scale = v as f32;
                    }
                    self.aligment_scale as f64
                })
                .prefix("Aligment scale"),
            );

            ui.add(
                egui::Slider::from_get_set(0.0..=1.0, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.separation_scale = v as f32;
                    }
                    self.separation_scale as f64
                })
                .prefix("Separation scale"),
            );
            ui.add(
                egui::Slider::from_get_set(0.0..=1.0, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.repulsion_margin = v as f32;
                    }
                    self.repulsion_margin as f64
                })
                .prefix("Repulsion margin"),
            );

            ui.add(
                egui::Slider::from_get_set(0.0..=1.0, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.repulsion_strength = v as f32;
                    }
                    self.repulsion_strength as f64
                })
                .prefix("Repulsion strength"),
            );
        });
    }
}
