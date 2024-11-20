tonic::include_proto!("simulation");

use crate::bodies::SimulationState;
use bevy::prelude::{Commands, Res, ResMut, Resource};
use bevy_tokio_tasks::*;
use colored::*;
use sim_server::{Sim, SimServer};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tonic::{transport::Server, Request, Response, Status};

#[derive(Default, Clone, Resource)]
pub struct SimulationService {
    pub state: Arc<Mutex<SimulationState>>,
}

pub fn setup_server(
    sim_state: Res<'_, SimulationState>,
    runtime: ResMut<'_, TokioTasksRuntime>,
    mut commands: Commands,
) {
    let service = SimulationService {
        state: Arc::new(Mutex::new(SimulationState {
            body_attributes: sim_state.body_attributes.clone(),
        })),
    };
    commands.insert_resource(service.clone());

    let addr: SocketAddr = "0.0.0.0:50051".parse().unwrap();
    runtime.spawn_background_task(move |_ctx| async move {
        let service = service.clone();
        let addr = addr.clone();
        Server::builder()
            .add_service(SimServer::new(service))
            .serve(addr)
            .await
            .expect("[Server] Failed to start");
    });
}

#[tonic::async_trait]
impl Sim for SimulationService {
    async fn replies(&self, _request: Request<SimReq>) -> Result<Response<SimResponse>, Status> {
        println!("{} Responded with: \n {:?}", "[Server]".green(), self.state);
        let state = self.state.lock().unwrap();
        let mut body_velocity_position: Vec<BodyAttributes> = vec![];
        let mut body_state: BodyAttributes;
        for body in &state.body_attributes {
            //TODO: Make this a little less heinous
            body_state = BodyAttributes {
                body_id: body.id.0,
                velocity: Some(Vec2D {
                    x: body.velocity.x,
                    y: body.velocity.y,
                }),
                position: Some(Vec2D {
                    x: body.position.x,
                    y: body.position.y,
                }),
            };
            body_velocity_position.push(body_state);
        }

        let response = SimResponse {
            bodies: body_velocity_position,
        };

        Ok(Response::new(response))
    }
}
