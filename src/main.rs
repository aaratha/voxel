use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*};
use bevy_math::prelude::*;

const PLAYER_SPEED: f32 = 3.0;
const JUMP_VELOCITY: f32 = 3.0;
const GRAVITY: f32 = -12.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (bounce_player, player_controller, camera_controller),
        )
        .run();
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

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((Camera3dBundle {
        camera: Camera {
            hdr: true, // 1. HDR is required for bloom
            ..default()
        },
        tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
        transform: Transform::from_xyz(10.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    },));

    // Adding a directional light with shadows
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let material_emissive1 = materials.add(StandardMaterial {
        emissive: Color::rgb(13.99, 5.32, 2.0), // 4. Put something bright in a dark environment to see the effect
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

    // Example instructions
    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );

    // Adding a platform
    let platform_mesh = meshes.add(Mesh::from(Plane3d {
        normal: Direction3d::new(Vec3::Y).unwrap(),
    }));
    let platform_material = materials.add(StandardMaterial {
        base_color: Color::GRAY,
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: platform_mesh,
        material: platform_material,
        transform: Transform::from_xyz(0.0, -0.5, 0.0),
        ..default()
    });
}

// ------------------------------------------------------------------------------------------------

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

fn bounce_player(time: Res<Time>, mut query: Query<&mut Transform, With<Bouncing>>) {}
