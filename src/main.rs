use bevy::{
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    winit::WinitPlugin,
};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use bodies::StartBodiesEvent;
use std::{env, str::FromStr};
mod bodies;
mod server;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut app = App::new();

    if args.contains(&String::from_str("headless").unwrap()) {
        app.add_plugins(DefaultPlugins.build().set(RenderPlugin {
            synchronous_pipeline_compilation: true,
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: None,
                ..default()
            }),
        }));
        println!("[Simulation] Running in headless mode");
    } else {
        app.add_plugins(DefaultPlugins)
            .add_systems(Startup, setup_graphics)
            .add_systems(Update, camera_update)
            .add_plugins(RapierDebugRenderPlugin::default());
    }

    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(20.0))
        .add_plugins(ShapePlugin)
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .insert_resource(RapierConfiguration {
            timestep_mode: TimestepMode::Fixed {
                dt: 1.0 / 10.0,
                substeps: 1,
            },
            gravity: Vec2::new(0.0, 0.0),
            physics_pipeline_active: true,
            query_pipeline_active: true,
            scaled_shape_subdivision: 10, // Set subdivision level for scaled shapes
            force_update_from_transform_changes: true, // Force updates based on transform changes
        })
        .insert_resource(bodies::parse_config())
        .add_systems(Startup, server::setup_server)
        .add_systems(Update, bodies::gravity_update)
        .add_systems(Update, bodies::vector_update.after(bodies::gravity_update))
        .add_event::<bodies::StartBodiesEvent>()
        .add_systems(
            Update,
            (
                bodies::despawn_everything.run_if(on_event::<StartBodiesEvent>()),
                bodies::spawn_bodies.run_if(on_event::<StartBodiesEvent>()),
                bodies::setup_vectors.run_if(on_event::<StartBodiesEvent>()),
                bodies::trigger_start,
            ),
        )
        .run();
}
fn setup_graphics(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 1.5,
            ..default()
        },
        ..default()
    });
}

fn camera_update(
    q_bodies: Query<(&Transform, &ColliderMassProperties), With<Velocity>>,
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<Velocity>)>,
) {
    let mut biggest_pos: Vec3 = Vec3::ZERO;
    let mut biggest_mass: f32 = 0.0;

    for (transform, mass) in q_bodies.iter() {
        let current = match mass {
            ColliderMassProperties::Mass(some_mass) => Some(*some_mass),
            _ => Some(0.0),
        };

        if current.unwrap() > biggest_mass {
            biggest_pos = transform.translation;
            biggest_mass = current.unwrap()
        }
    }
    for mut transform in camera.iter_mut() {
        biggest_pos.z = transform.translation.z;
        transform.translation = biggest_pos;
    }
}
