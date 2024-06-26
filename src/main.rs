//! Demonstrates depth of field (DOF).
//!
//! The depth of field effect simulates the blur that a real camera produces on
//! objects that are out of focus.
//!
//! The test scene is inspired by [a blog post on depth of field in Unity].
//! However, the technique used in Bevy has little to do with that blog post,
//! and all the assets are original.
//!
//! [a blog post on depth of field in Unity]: https://catlikecoding.com/unity/tutorials/advanced-rendering/depth-of-field/

use std::time::Duration;

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        dof::{self, DepthOfFieldMode, DepthOfFieldSettings},
        tonemapping::Tonemapping,
    },
    prelude::*,
};

use bevy::{
    math::Affine2,
    render::texture::{
        ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
    },
};

const FOCAL_DISTANCE_SPEED: f32 = 0.05;
const APERTURE_F_STOP_SPEED: f32 = 0.01;
const MIN_FOCAL_DISTANCE: f32 = 0.01;
const MIN_APERTURE_F_STOPS: f32 = 0.05;

const PLAYER_SPEED: f32 = 24.0;
const PLAYER_LERP_SPEED: f32 = 0.1;
const PLAYER_ROTATION_SPEED: f32 = 0.2;
const JUMP_VELOCITY: f32 = 25.0;
const GRAVITY: f32 = -100.;

/// A resource that stores the settings that the user can change.
#[derive(Clone, Copy, Resource)]
struct AppSettings {
    focal_distance: f32,
    aperture_f_stops: f32,
    mode: Option<DepthOfFieldMode>,
}

#[derive(Component)]
struct Position {
    current: Vec3,
    target: Vec3,
    vertical_velocity: f32,
}

#[derive(Component)]
struct Rotation {
    pub radians_y: f32,
}

#[derive(Component)]
struct Checks {
    is_moving: bool,
}

#[derive(Bundle)]
struct PlayerBundle {
    position: Position,
    rotation: Rotation,
    #[bundle()]
    pbr: SceneBundle,
    checks: Checks,
}

impl PlayerBundle {
    fn new(scene: Handle<Scene>) -> Self {
        Self {
            position: Position {
                current: Vec3::ZERO,
                target: Vec3::ZERO,
                vertical_velocity: 0.0,
            },
            rotation: Rotation { radians_y: 0.0 },
            pbr: SceneBundle {
                scene: scene,
                transform: Transform::from_scale(Vec3::splat(0.012)),
                ..default()
            },
            checks: Checks { is_moving: false },
        }
    }
}

#[derive(Resource)]
struct Animations {
    animations: Vec<AnimationNodeIndex>,
    #[allow(dead_code)]
    graph: Handle<AnimationGraph>,
}

fn main() {
    App::new()
        .init_resource::<AppSettings>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Depth of Field Example".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup, update_dof_settings))
        .add_systems(
            Update,
            (
                // adjust_focus,
                player_controller,
                camera_controller,
                animation_controller,
                setup_scene_once_loaded,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    app_settings: Res<AppSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut graph = AnimationGraph::new();
    let animations = graph
        .add_clips(
            [
                GltfAssetLabel::Animation(2).from_asset("models/Fox.glb"),
                GltfAssetLabel::Animation(1).from_asset("models/Fox.glb"),
                GltfAssetLabel::Animation(0).from_asset("models/Fox.glb"),
            ]
            .into_iter()
            .map(|path| asset_server.load(path)),
            1.0,
            graph.root,
        )
        .collect();

    // Insert a resource with the current scene information
    let graph = graphs.add(graph);
    commands.insert_resource(Animations {
        animations,
        graph: graph.clone(),
    });

    // Load all required textures with settings to repeat
    let ambient_occlusion_texture =
        asset_server.load("textures/Grass 001 1K PNG/Grass001_1K-PNG_AmbientOcclusion.png");
    let color_texture = asset_server.load_with_settings(
        "textures/Grass 001 1K PNG/Grass001_1K-PNG_Color.png",
        |s: &mut _| {
            *s = ImageLoaderSettings {
                sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    ..default()
                }),
                ..default()
            }
        },
    );
    let normal_gl_texture = asset_server.load_with_settings(
        "textures/Grass 001 1K PNG/Grass001_1K-PNG_NormalGL.png",
        |s: &mut _| {
            *s = ImageLoaderSettings {
                sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    ..default()
                }),
                ..default()
            }
        },
    );
    let roughness_texture = asset_server.load_with_settings(
        "textures/Grass 001 1K PNG/Grass001_1K-PNG_Roughness.png",
        |s: &mut _| {
            *s = ImageLoaderSettings {
                sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    ..default()
                }),
                ..default()
            }
        },
    );

    let grass_material = materials.add(StandardMaterial {
        base_color_texture: Some(color_texture.clone()),
        occlusion_texture: Some(ambient_occlusion_texture.clone()),
        normal_map_texture: Some(normal_gl_texture.clone()),
        metallic_roughness_texture: Some(roughness_texture.clone()),
        uv_transform: Affine2::from_scale(Vec2::new(5., 5.)), // Repeat texture 5 times in each direction
        ..Default::default()
    });

    // Spawn the camera. Enable HDR and bloom, as that highlights the depth of
    // field effect.
    let mut camera = commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 2.5, 8.25).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            hdr: true,
            ..default()
        },
        tonemapping: Tonemapping::TonyMcMapface,
        ..default()
    });
    camera.insert(BloomSettings::NATURAL);

    // Insert the depth of field settings.
    if let Some(dof_settings) = Option::<DepthOfFieldSettings>::from(*app_settings) {
        camera.insert(dof_settings);
    }

    // Spawning the player entity
    commands.spawn(PlayerBundle::new(
        asset_server.load("models/Fox.glb#Scene0"),
    ));

    // Adding a directional light with shadows
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Adding a platform
    let platform_mesh = meshes.add(Mesh::from(Plane3d {
        normal: Dir3::new(Vec3::Y).unwrap(),
        half_size: Vec2::new(30.0, 30.0),
    }));

    commands.spawn(PbrBundle {
        mesh: platform_mesh,
        material: grass_material,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
}

fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        let mut transitions = AnimationTransitions::new();

        transitions
            .play(&mut player, animations.animations[2], Duration::ZERO)
            .set_speed(2.);

        commands
            .entity(entity)
            .insert(animations.graph.clone())
            .insert(transitions);
    }
}

/// Adjusts the focal distance and f-number per user inputs.
fn adjust_focus(input: Res<ButtonInput<KeyCode>>, mut app_settings: ResMut<AppSettings>) {
    // Change the focal distance if the user requested.
    let distance_delta = if input.pressed(KeyCode::ArrowDown) {
        -FOCAL_DISTANCE_SPEED
    } else if input.pressed(KeyCode::ArrowUp) {
        FOCAL_DISTANCE_SPEED
    } else {
        0.0
    };

    app_settings.focal_distance =
        (app_settings.focal_distance + distance_delta).max(MIN_FOCAL_DISTANCE);

    println!("Focal distance: {}", app_settings.focal_distance);
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            focal_distance: 11.,
            aperture_f_stops: 1.0 / 30.0,
            mode: Some(DepthOfFieldMode::Bokeh),
        }
    }
}

/// Writes the depth of field settings into the camera.
fn update_dof_settings(
    mut commands: Commands,
    view_targets: Query<Entity, With<Camera>>,
    app_settings: Res<AppSettings>,
) {
    let dof_settings: Option<DepthOfFieldSettings> = (*app_settings).into();
    for view in view_targets.iter() {
        match dof_settings {
            None => {
                commands.entity(view).remove::<DepthOfFieldSettings>();
            }
            Some(dof_settings) => {
                commands.entity(view).insert(dof_settings);
            }
        }
    }
}

impl From<AppSettings> for Option<DepthOfFieldSettings> {
    fn from(app_settings: AppSettings) -> Self {
        app_settings.mode.map(|mode| DepthOfFieldSettings {
            mode,
            focal_distance: app_settings.focal_distance,
            aperture_f_stops: app_settings.aperture_f_stops,
            max_depth: 14.0,
            ..default()
        })
    }
}
fn player_controller(
    time: Res<Time>,
    mut player_query: Query<(&mut Position, &mut Rotation, &mut Transform, &mut Checks)>,

    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    for (mut position, mut rotation, mut transform, mut player) in player_query.iter_mut() {
        let dt = time.delta_seconds();

        let mut movement = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyW) {
            movement += Vec3::new(-PLAYER_SPEED, 0.0, 0.0);
            player.is_moving = true;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            movement += Vec3::new(PLAYER_SPEED, 0.0, 0.0);
            player.is_moving = true;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            movement += Vec3::new(0.0, 0.0, PLAYER_SPEED);
            player.is_moving = true;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            movement += Vec3::new(0.0, 0.0, -PLAYER_SPEED);
            player.is_moving = true;
        }

        // Normalize movement vector if needed
        if movement.length_squared() > 0.0 {
            movement = movement.normalize();
        } else {
            player.is_moving = false;
        }

        // Apply speed to movement vector
        movement *= PLAYER_SPEED * dt;

        // Update target position
        position.target += movement * PLAYER_SPEED * dt;

        // Update rotation to face movement direction
        if movement.length_squared() > 0.0 {
            rotation.radians_y = movement.x.atan2(movement.z);
        }

        // Update current position
        position.current = position.current.lerp(position.target, PLAYER_LERP_SPEED);
        transform.translation = position.current;

        // Apply rotation to transform
        let angle = Quat::from_rotation_y(rotation.radians_y);
        transform.rotation = transform.rotation.lerp(angle, PLAYER_ROTATION_SPEED);

        // Vertical movement (jump)
        position.vertical_velocity += GRAVITY * dt;
        position.target.y += position.vertical_velocity * dt;

        if keyboard_input.just_pressed(KeyCode::Space) && position.target.y <= 0.0 {
            position.vertical_velocity = JUMP_VELOCITY;
            position.target.y = 0.1;
        }

        if position.target.y < 0.0 {
            position.target.y = 0.0;
            position.vertical_velocity = 0.0;
        }
    }
}

fn animation_controller(
    mut animation_players: Query<&mut AnimationPlayer>,
    mut player_query: Query<&mut Checks>,
    animations: Res<Animations>,
) {
    for mut player in player_query.iter_mut() {
        for mut animation_player in animation_players.iter_mut() {
            // Check if the 'W' key is just pressed or just released
            if player.is_moving {
                animation_player.stop(animations.animations[2].clone());
                animation_player
                    .play(animations.animations[0].clone())
                    .set_speed(3.0) // Adjust speed as needed
                    .repeat();
            } else {
                // Stop any current animations and play the idle animation
                animation_player.stop(animations.animations[0].clone());
                animation_player
                    .play(animations.animations[2].clone())
                    .repeat();
            }
        }
    }
}

fn camera_controller(
    player_query: Query<&Position>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    if let Ok(position) = player_query.get_single() {
        for mut camera_transform in camera_query.iter_mut() {
            camera_transform.translation =
                Vec3::new(position.current.x + 8.0, 8.0, position.current.z);
            camera_transform.look_at(position.current, Vec3::Y);
        }
    }
}
