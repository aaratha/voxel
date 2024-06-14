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

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        dof::{self, DepthOfFieldMode, DepthOfFieldSettings},
        tonemapping::Tonemapping,
    },
    math::Direction3d,
    pbr::Lightmap,
    prelude::*,
    render::camera::PhysicalCameraParameters,
};

const FOCAL_DISTANCE_SPEED: f32 = 0.05;
const APERTURE_F_STOP_SPEED: f32 = 0.01;
const MIN_FOCAL_DISTANCE: f32 = 0.01;
const MIN_APERTURE_F_STOPS: f32 = 0.05;

const PLAYER_SPEED: f32 = 3.0;
const JUMP_VELOCITY: f32 = 3.0;
const GRAVITY: f32 = -12.;

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
struct Bouncing;

#[derive(Bundle)]
struct PlayerBundle {
    position: Position,
    #[bundle()]
    pbr: PbrBundle,
}

impl PlayerBundle {
    fn new(mesh: Handle<Mesh>, material: Handle<StandardMaterial>) -> Self {
        Self {
            position: Position {
                current: Vec3::ZERO,
                target: Vec3::ZERO,
                vertical_velocity: 0.0,
            },
            pbr: PbrBundle {
                mesh,
                material,
                transform: Transform::default(),
                ..default()
            },
        }
    }
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
        .add_systems(Startup, setup)
        .add_systems(Update, tweak_scene)
        .add_systems(
            Update,
            (
                adjust_focus,
                update_dof_settings,
                player_controller,
                camera_controller,
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
) {
    // Spawn the camera. Enable HDR and bloom, as that highlights the depth of
    // field effect.
    let mut camera = commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 4.5, 8.25).looking_at(Vec3::ZERO, Vec3::Y),
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

    let material_emissive1 = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5).into(),
        emissive: Color::srgb(1., 1., 1.).into(), // 4. Put something bright in a dark environment to see the effect
        ..default()
    });

    let mesh = meshes.add(Mesh::from(Sphere { radius: 0.5 }));

    // Spawning the player entity
    commands.spawn(PlayerBundle::new(mesh.clone(), material_emissive1.clone()));

    let material = material_emissive1.clone();

    commands.spawn((
        PbrBundle {
            mesh: mesh.clone(),
            material,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Bouncing,
    ));

    // Adding a platform
    let platform_mesh = meshes.add(Mesh::from(Plane3d {
        normal: Dir3::new(Vec3::Y).unwrap(),
        half_size: Vec2::new(5.0, 5.0),
    }));
    let platform_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5).into(),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: platform_mesh,
        material: platform_material,
        transform: Transform::from_xyz(0.0, -0.5, 0.0),
        ..default()
    });
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
            focal_distance: 13.0,
            aperture_f_stops: 1.0 / 40.0,
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

/// Makes one-time adjustments to the scene that can't be encoded in glTF.
fn tweak_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut DirectionalLight, Changed<DirectionalLight>>,
    mut named_entities: Query<
        (Entity, &Name, &Handle<StandardMaterial>),
        (With<Handle<Mesh>>, Without<Lightmap>),
    >,
) {
    // Turn on shadows.
    for mut light in lights.iter_mut() {
        light.shadows_enabled = true;
    }

    // Add a nice lightmap to the circuit board.
    for (entity, name, material) in named_entities.iter_mut() {
        if &**name == "CircuitBoard" {
            materials.get_mut(material).unwrap().lightmap_exposure = 10000.0;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("models/CircuitBoardLightmap.hdr"),
                ..default()
            });
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
    mut player_query: Query<(&mut Position, &mut Transform)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    for (mut position, mut transform) in player_query.iter_mut() {
        let dt = time.delta_seconds();

        if keyboard_input.pressed(KeyCode::KeyW) {
            position.target += Vec3::new(-PLAYER_SPEED, 0.0, 0.0) * dt;
        }

        if keyboard_input.pressed(KeyCode::KeyA) {
            position.target += Vec3::new(0.0, 0.0, PLAYER_SPEED) * dt;
        }

        if keyboard_input.pressed(KeyCode::KeyS) {
            position.target += Vec3::new(PLAYER_SPEED, 0.0, 0.0) * dt;
        }

        if keyboard_input.pressed(KeyCode::KeyD) {
            position.target += Vec3::new(0.0, 0.0, -PLAYER_SPEED) * dt;
        }

        position.current = position.current.lerp(position.target, 0.1);
        transform.translation = position.current;
        position.vertical_velocity += GRAVITY * dt;
        position.target.y += position.vertical_velocity * dt;

        // Jump
        if keyboard_input.just_pressed(KeyCode::Space) && position.target.y <= 0.0 {
            position.vertical_velocity = JUMP_VELOCITY;
            position.target.y = 0.1; // Small offset to prevent multiple jumps
        }

        // Prevent falling below the platform
        if position.target.y < 0.0 {
            position.target.y = 0.0;
            position.vertical_velocity = 0.0;
        }

        position.current = position.current.lerp(position.target, 0.1);

        transform.translation = position.current;
    }
}

fn camera_controller(
    time: Res<Time>,
    player_query: Query<&Position>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    if let Ok(position) = player_query.get_single() {
        for mut camera_transform in camera_query.iter_mut() {
            camera_transform.translation =
                Vec3::new(position.current.x + 10.0, 7.0, position.current.z);
            camera_transform.look_at(position.current, Vec3::Y);
        }
    }
}
