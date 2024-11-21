use bevy::{math::bool, prelude::*};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use rapier2d::na::Vector2;
use serde::Deserialize;
use std::{fs::File, io::Read};

use crate::server::SimulationService;

#[derive(Event)]
pub struct StartBodiesEvent;

#[derive(Debug, Clone, Deserialize)]
pub struct BodyAttributes {
    pub id: BodyId,
    pub radius: f32,
    pub restitution: f32,
    pub mass: f32,
    pub velocity: VectorStruct,
    pub position: VectorStruct,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VectorStruct {
    pub x: f32,
    pub y: f32,
}

#[derive(Default, Clone, Deserialize, Debug, Component, Copy)]
pub struct BodyId(pub u32);

#[derive(Default, Debug, Resource, Deserialize)]
pub struct SimulationState {
    //I would love this to be a tuple struct
    //but the toml parsing likes it this way
    pub body_attributes: Vec<BodyAttributes>,
}

pub fn despawn_everything(
    mut commands: Commands,
    query_body: Query<Entity, Or<(With<RigidBody>, With<Collider>)>>,
    query_path: Query<Entity, With<Path>>,
    mut event_reader: EventReader<StartBodiesEvent>,
) {
    for _ in event_reader.read() {
        for body_entity in query_body.iter() {
            commands.entity(body_entity).despawn();
        }
        for path_entity in query_path.iter() {
            commands.entity(path_entity).despawn();
        }
    }
}

pub fn trigger_start(
    mut event_writer: EventWriter<StartBodiesEvent>,
    service: ResMut<SimulationService>,
) {
    let mut reset = service.reset.lock().unwrap();
    if *reset {
        event_writer.send(StartBodiesEvent);
        println!("[Sim] Starting!");
        *reset = false;
    }
}

pub fn parse_config() -> SimulationState {
    let mut file = File::open("config.toml").unwrap();
    let mut configuration = String::new();
    file.read_to_string(&mut configuration).unwrap();
    toml::from_str(&configuration).unwrap()
}

pub fn spawn_bodies(
    mut commands: Commands,
    bodies: Res<SimulationState>,
    mut event_reader: EventReader<StartBodiesEvent>,
) {
    for _ in event_reader.read() {
        let bodies_iter = &bodies.body_attributes;
        for body in bodies_iter {
            commands
                .spawn(RigidBody::Dynamic)
                .insert(Collider::ball(body.radius))
                .insert(Restitution::coefficient(body.restitution))
                .insert(ColliderMassProperties::Mass(body.mass))
                .insert(ExternalForce::default())
                .insert(body.id)
                .insert(Velocity {
                    linvel: Vec2::new(body.velocity.x, body.velocity.y),
                    ..default()
                })
                .insert(TransformBundle::from(Transform::from_xyz(
                    body.position.x,
                    body.position.y,
                    0.0,
                )));
        }
    }
}

pub fn gravitational_force(
    mass1: f32,
    mass2: f32,
    position1: Vector2<f32>,
    position2: Vector2<f32>,
) -> Vector2<f32> {
    let r = position2 - position1;
    let direction = r.norm();
    let f_mag = 1000.0 * ((mass1 * mass2) / direction.powi(2));
    r.normalize() * f_mag
}

pub fn gravity_update(
    mut bodies: Query<(
        &ColliderMassProperties,
        &Transform,
        &mut ExternalForce,
        &BodyId,
        &Velocity,
    )>,
    service: ResMut<SimulationService>,
) {
    let mut combinations = bodies.iter_combinations_mut::<2>();
    while let Some([body1, body2]) = combinations.fetch_next() {
        let (mass_properties_1, translation1, mut ex_force_1, body_id_1, velocity_1) = body1;
        let (mass_properties_2, translation2, mut ex_force_2, body_id_2, velocity_2) = body2;

        //now this is just awful
        let mass1 = match mass_properties_1 {
            ColliderMassProperties::Mass(mass) => Some(*mass),
            _ => Some(0.0),
        };
        let mass2 = match mass_properties_2 {
            ColliderMassProperties::Mass(mass) => Some(*mass),
            _ => Some(0.0),
        };
        let f_1_2 = gravitational_force(
            mass1.unwrap(),
            mass2.unwrap(),
            translation1.translation.truncate().into(),
            translation2.translation.truncate().into(),
        );
        let f_2_1 = -f_1_2;
        ex_force_1.force = f_1_2.into();
        ex_force_2.force = f_2_1.into();

        let mut state = service.state.lock().unwrap();

        //shitty imperative code is imperative
        for body in &mut state.body_attributes {
            if body.id.0 == body_id_1.0 {
                body.position = VectorStruct {
                    x: translation1.translation.truncate().x,
                    y: translation1.translation.truncate().y,
                };
                body.velocity = VectorStruct {
                    x: velocity_1.linvel.x,
                    y: velocity_1.linvel.y,
                };
            }
            if body.id.0 == body_id_2.0 {
                body.position = VectorStruct {
                    x: translation2.translation.truncate().x,
                    y: translation2.translation.truncate().y,
                };
                body.velocity = VectorStruct {
                    x: velocity_2.linvel.x,
                    y: velocity_2.linvel.y,
                };
            }
        }
    }
}

pub fn setup_vectors(mut commands: Commands, query_bodies: Query<&Transform>) {
    for _ in query_bodies.iter() {
        let line = shapes::Line(Vec2::ZERO, Vec2::new(0.0, 0.0));
        commands.spawn((
            ShapeBundle {
                path: GeometryBuilder::build_as(&line),
                ..default()
            },
            Stroke::new(Color::WHITE, 5.0), // Spawn in lines
        ));
    }
}

pub fn vector_update(query_body: Query<(&Transform, &Velocity)>, mut query_path: Query<&mut Path>) {
    for ((transform, velocity), mut path) in query_body.iter().zip(query_path.iter_mut()) {
        let center_of_mass = transform.translation.truncate();
        let vel = velocity.linvel;
        let new_line = shapes::Line(center_of_mass, center_of_mass + vel);
        *path = ShapePath::build_as(&new_line);
    }
}
