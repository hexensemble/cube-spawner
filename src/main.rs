use avian3d::prelude::*;
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_framepace::*;
use rand::prelude::*;

const FPS_CAP: f64 = 30.0;
const CUBE_SOUND: &str = "cube.wav";
const CUBE_DESPAWN_TIME: f32 = 5.0;
const CAMERA_ANGULAR_SPEED: f32 = 1.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            FramepacePlugin,
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .init_resource::<Sound>()
        .init_resource::<CubeCounter>()
        .add_systems(
            Startup,
            (load_sounds, setup_framepace, setup_world, setup_ui),
        )
        .add_systems(
            Update,
            (
                cube_timer,
                handle_collison_event,
                space_bar,
                move_camera,
                update_cube_count_text,
                update_fps_text,
            ),
        )
        .add_observer(spawn_cube)
        .run();
}

fn load_sounds(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Sound(asset_server.load(CUBE_SOUND)));
}

fn setup_framepace(mut settings: ResMut<FramepaceSettings>) {
    settings.limiter = Limiter::from_framerate(FPS_CAP);
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Floor
    commands.spawn((
        RigidBody::Static,
        Collider::cylinder(4.0, 0.1),
        Mesh3d(meshes.add(Cylinder::new(4.0, 0.1))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Dir3::Y),
    ));
}

fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: px(50.0),
            left: px(50.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(5.0),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(Text::new("Cube Count: "))
                .with_child((TextSpan::default(), CubeCountText));

            parent
                .spawn(Text::new("FPS: "))
                .with_child((TextSpan::default(), FPSText));

            parent.spawn(Text::new("Press Space to spawn a cube"));

            parent.spawn(Text::new("Press A or D to pan"));
        });
}

#[derive(Resource, Deref, Default)]
struct Sound(Handle<AudioSource>);

#[derive(Resource, Default)]
struct CubeCounter(i32);

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Cube;

#[derive(Component)]
struct Lifetime {
    timer: Timer,
}

#[derive(Component)]
struct CubeCountText;

#[derive(Component)]
struct FPSText;

#[derive(Event)]
struct SpawnCubeEvent;

fn spawn_cube(
    _trigger: On<SpawnCubeEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cube_counter: ResMut<CubeCounter>,
) {
    let mut rng = rand::rng();

    let size = rng.random_range(0.1..1.0);

    let pos_min = Vec3::new(-4.0, 0.0, -4.0);
    let pos_max = Vec3::new(4.0, 8.0, 4.0);
    let position = Vec3::new(
        rng.random_range(pos_min.x..pos_max.x),
        rng.random_range(pos_min.y..pos_max.y),
        rng.random_range(pos_min.z..pos_max.z),
    );

    let ang_min = Vec3::splat(-4.0);
    let ang_max = Vec3::splat(4.0);
    let angular_velocity = Vec3::new(
        rng.random_range(ang_min.x..ang_max.x),
        rng.random_range(ang_min.y..ang_max.y),
        rng.random_range(ang_min.z..ang_max.z),
    );

    commands.spawn((
        Cube,
        RigidBody::Dynamic,
        Collider::cuboid(size, size, size),
        AngularVelocity(angular_velocity),
        Mesh3d(meshes.add(Cuboid::from_length(size))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(position.x, position.y, position.z),
        CollisionEventsEnabled,
        Lifetime {
            timer: Timer::from_seconds(CUBE_DESPAWN_TIME, TimerMode::Once),
        },
    ));

    cube_counter.0 += 1;
}

fn cube_timer(
    mut commands: Commands,
    cubes: Query<(Entity, &mut Lifetime), With<Cube>>,
    mut cube_counter: ResMut<CubeCounter>,
    time: Res<Time>,
) {
    for (entity, mut lifetime) in cubes {
        // Advance timer
        lifetime.timer.tick(time.delta());

        // Check if timer ended
        if lifetime.timer.is_finished() {
            commands.entity(entity).despawn();

            cube_counter.0 -= 1;
        }
    }
}

fn handle_collison_event(
    mut commands: Commands,
    mut collision_reader: MessageReader<CollisionStart>,
    sound: Res<Sound>,
) {
    for event in collision_reader.read() {
        println!("{} and {} collided.", event.collider1, event.collider2);

        commands.spawn((AudioPlayer::new(sound.clone()), PlaybackSettings::DESPAWN));
    }
}

fn space_bar(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>) {
    if keyboard_input.just_released(KeyCode::Space) {
        commands.trigger(SpawnCubeEvent);
    }
}

fn move_camera(
    mut camera_transform: Single<&mut Transform, With<MainCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let mut angle = 0.0;

    if keyboard_input.pressed(KeyCode::KeyA) {
        angle += CAMERA_ANGULAR_SPEED * time.delta_secs();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        angle -= CAMERA_ANGULAR_SPEED * time.delta_secs();
    }

    if angle != 0.0 {
        let rotation = Quat::from_rotation_y(angle);

        camera_transform.translation = rotation * camera_transform.translation;
        camera_transform.look_at(Vec3::ZERO, Dir3::Y);
    }
}

fn update_cube_count_text(
    spans: Query<&mut TextSpan, With<CubeCountText>>,
    cube_counter: Res<CubeCounter>,
) {
    for mut span in spans {
        **span = format!("{}", cube_counter.0);
    }
}

fn update_fps_text(spans: Query<&mut TextSpan, With<FPSText>>, diagnostics: Res<DiagnosticsStore>) {
    for mut span in spans {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(value) = fps.smoothed()
        {
            **span = format!("{:.0}", value);
        }
    }
}
