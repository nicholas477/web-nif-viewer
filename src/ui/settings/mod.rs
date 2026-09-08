#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::*;

/// Draws the keyboard shortcut settings window.
pub fn draw_keybindings(
	ctx: &bevy_egui::egui::Context,
	state: &mut crate::UIState,
	keys: &bevy::prelude::ButtonInput<bevy::prelude::KeyCode>,
	mouse_buttons: &bevy::prelude::ButtonInput<bevy::prelude::MouseButton>,
	pan_orbit: &mut crate::camera::PanOrbitCamera,
) {
	if !state.top_panel.show_keybindings {
		return;
	}

	let mut open = state.top_panel.show_keybindings;
	let mouse_binding_before_click = state.top_panel.capturing_mouse_binding;
	let mut changed = false;
	bevy_egui::egui::Window::new("Key Bindings")
		.open(&mut open)
		.resizable(false)
		.show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.label("Frame mesh");
				let label = if state.top_panel.capturing_frame_mesh_key {
					"Press a key...".to_string()
				} else {
					format!("{:?}", state.key_bindings.frame_mesh)
				};
				if ui.button(label).clicked() {
					state.top_panel.capturing_frame_mesh_key = true;
				}
			});

			mouse_binding_row(
				ui,
				"Rotate",
				Some(state.key_bindings.orbit),
				crate::MouseBinding::Orbit,
				&mut state.top_panel.capturing_mouse_binding,
			);
			mouse_binding_row(
				ui,
				"Pan",
				Some(state.key_bindings.pan),
				crate::MouseBinding::Pan,
				&mut state.top_panel.capturing_mouse_binding,
			);
			mouse_binding_row(
				ui,
				"Zoom",
				state.key_bindings.zoom,
				crate::MouseBinding::Zoom,
				&mut state.top_panel.capturing_mouse_binding,
			);

			if state.top_panel.capturing_frame_mesh_key
				&& let Some(key) = keys.get_just_pressed().next().copied()
			{
				state.key_bindings.frame_mesh = key;
				state.top_panel.capturing_frame_mesh_key = false;
				changed = true;
			}

			if mouse_binding_before_click.is_some()
				&& let Some(button) = mouse_buttons.get_just_pressed().next().copied()
				&& let Some(binding) = state.top_panel.capturing_mouse_binding
			{
				match binding {
					crate::MouseBinding::Orbit => {
						state.key_bindings.orbit = button;
						pan_orbit.button_orbit = button;
					}
					crate::MouseBinding::Pan => {
						state.key_bindings.pan = button;
						pan_orbit.button_pan = button;
					}
					crate::MouseBinding::Zoom => {
						state.key_bindings.zoom = Some(button);
						pan_orbit.button_zoom = Some(button);
					}
				}
				state.top_panel.capturing_mouse_binding = None;
				changed = true;
			}

			if ui.button("Reset defaults").clicked() {
				state.key_bindings = crate::KeyBindings::default();
				state.top_panel.capturing_frame_mesh_key = false;
				state.top_panel.capturing_mouse_binding = None;
				pan_orbit.button_orbit = state.key_bindings.orbit;
				pan_orbit.button_pan = state.key_bindings.pan;
				pan_orbit.button_zoom = state.key_bindings.zoom;
				changed = true;
			}
		});
	state.top_panel.show_keybindings = open;
	if !open {
		state.top_panel.capturing_frame_mesh_key = false;
		state.top_panel.capturing_mouse_binding = None;
	}
	if changed {
		save_keybindings(&state.key_bindings);
	}
}

fn mouse_binding_row(
	ui: &mut bevy_egui::egui::Ui,
	label: &str,
	binding: Option<bevy::prelude::MouseButton>,
	action: crate::MouseBinding,
	capturing: &mut Option<crate::MouseBinding>,
) {
	ui.horizontal(|ui| {
		ui.label(label);
		let button_label = if *capturing == Some(action) {
			"Click a mouse button...".to_string()
		} else {
			binding
				.map(|button| format!("{button:?}"))
				.unwrap_or_else(|| "Scroll wheel".to_string())
		};
		if ui.button(button_label).clicked() {
			*capturing = Some(action);
		}
	});
}
