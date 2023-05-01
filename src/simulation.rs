use oxyde::egui; 

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimulationParametersUniformBufferContent {
    pub boids_count: u32,
    pub delta_t: f32,
    pub view_radius: f32,
    pub cohesion_scale: f32,
    pub aligment_scale: f32,
    pub separation_scale: f32,
    pub grid_count: u32,
}

impl Default for SimulationParametersUniformBufferContent {
    fn default() -> Self {
        Self {
            boids_count: 256,
            delta_t: 0.04,
            view_radius: 0.05,
            cohesion_scale: 0.02,
            aligment_scale: 0.005,
            separation_scale: 0.05,
            grid_count: 4,
        }
    }
}

impl SimulationParametersUniformBufferContent {
    pub fn display_ui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Simulation settings").default_open(true).show(ui, |ui| {
            ui.add(
                egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.delta_t = v as f32;
                    }
                    self.delta_t as f64
                })
                .prefix("Delta t"),
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
                egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.cohesion_scale = v as f32;
                    }
                    self.cohesion_scale as f64
                })
                .prefix("Cohesion scale"),
            );

            ui.add(
                egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.aligment_scale = v as f32;
                    }
                    self.aligment_scale as f64
                })
                .prefix("Aligment scale"),
            );

            ui.add(
                egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.separation_scale = v as f32;
                    }
                    self.separation_scale as f64
                })
                .prefix("Separation scale"),
            );
        });
    }
    
}
