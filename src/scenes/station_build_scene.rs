use std::default::default;
use bevy::asset::AssetServer;
use egui::{Context, Ui};
use space_game::{Game, GameCommands, SchedulePlugin, GlobalStageStep, EguiContext, SceneType, RonAssetPlugin, RenderApi, InputSystem, KeyCode};
use space_render::add_game_render_plugins;
use space_core::{ecs::*, app::App};
use space_core::{serde::*, Camera};
use bevy::reflect::*;
use bevy::asset::*;
use space_assets::{GltfAssetLoader, SpaceAssetServer};


pub struct StationBuildMenu {}

impl SchedulePlugin for StationBuildMenu {
    fn get_name(&self) -> space_game::PluginName {
        space_game::PluginName::Text("Station build menu".into())
    }

    fn add_system(&self, app : &mut App) {

        app.add_plugin(RonAssetPlugin::<RonBlockDesc>{ ext: vec!["wall"], ..default() });

        app.add_system_set(SystemSet::on_enter(SceneType::StationBuilding)
            .with_system(init_station_build));

        app.add_system_set(
            SystemSet::on_update(SceneType::StationBuilding)
                .with_system(station_menu)
                .with_system(camera_movement));

    }
}

fn camera_movement(
    mut camera : ResMut<Camera>,
    input : Res<InputSystem>) {
    
    let right = camera.get_right();
    if input.get_key_state(KeyCode::W) {
        camera.pos = camera.pos + 0.01 * camera.up;
    }
    if input.get_key_state(KeyCode::S) {
        camera.pos = camera.pos - 0.01 * camera.up;
    }
    if input.get_key_state(KeyCode::A) {
        camera.pos = camera.pos - 0.01 * right;
    }
    if input.get_key_state(KeyCode::D) {
        camera.pos = camera.pos + 0.01 * right;
    }
}

#[derive(Default, Deserialize, TypeUuid, Debug, Clone)]
#[uuid = "fce6d1f5-4317-4077-b23e-6099747b08dd"]
struct RonBlockDesc {
    pub name : String,
    pub model_path : String
}



#[derive(Resource, Default)]
struct StationBlocks {
    pub panels : Vec<Handle<RonBlockDesc>>,

    pub active_block : Option<RonBlockDesc>,
    pub active_entity : Option<Entity>

}


fn station_menu(
    mut commands : Commands,
    ctx : Res<EguiContext>,
    mut panels : ResMut<StationBlocks>,
    blocks : Res<Assets<RonBlockDesc>>,
    mut asset_server : ResMut<SpaceAssetServer>,
    render : Res<RenderApi>
) {
    egui::SidePanel::left("Build panel").show(&ctx, |ui| {
        if let Some(block) = panels.active_block.as_ref() {
            ui.label(format!("Selected block: {}", block.name));
        } else {
            ui.label(format!("Selected block: None"));
        }
        ui.separator();


        ui.label("Blocks:");
        let mut panel_list = panels.panels.clone();
        for h in &panel_list {
            if let Some(block) = blocks.get(h) {
                if ui.button(&block.name).clicked() {
                    panels.active_block = Some(block.clone());

                    asset_server.wgpu_gltf_load_cmds(
                        &render.device,
                        block.model_path.clone(),
                        &mut commands
                    );
                }
            }
        }
    });
}

fn init_station_build(
    mut commands : Commands,
    mut assets : Res<AssetServer>,
    mut camera : ResMut<Camera>
) {
    let mut blocks = StationBlocks::default();
    blocks.panels.push(assets.load("ss13/walls_configs/metal_grid.wall"));
    commands.insert_resource(blocks);

    camera.pos.x = 0.0;
    camera.pos.y = 10.0;
    camera.pos.z = 0.0;

    camera.up.y = 0.0;
    camera.up.z = 1.0;
    camera.up.x = 0.0;

    camera.frw.x = 0.0;
    camera.frw.y = -1.0;
    camera.frw.z = 0.0;
}

