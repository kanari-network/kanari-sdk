use std::collections::{BTreeMap, HashSet, VecDeque};

use centauri::consensus::{CrossShardDispatch, CrossShardMessage, ShardId, ShardedDag};

fn single_authority() -> Vec<String> {
    vec!["auth1".to_string()]
}

fn routing_key_for_target(dag: &ShardedDag, target: ShardId) -> Vec<u8> {
    for candidate in 0u32..10_000 {
        let key = format!("simulation-route-key-{candidate}");
        if dag.route_payload(key.as_bytes()) == target {
            return key.into_bytes();
        }
    }
    panic!("failed to find routing key for target shard {target}");
}

pub struct MultiNodeSimulationHarness {
    nodes: BTreeMap<ShardId, ShardedDag>,
    blocked_routes: HashSet<(ShardId, ShardId)>,
    delayed_messages: VecDeque<CrossShardMessage>,
}

impl MultiNodeSimulationHarness {
    pub fn new(num_shards: ShardId) -> Self {
        let mut nodes = BTreeMap::new();
        for shard_id in 0..num_shards {
            nodes.insert(
                shard_id,
                ShardedDag::new(
                    shard_id,
                    num_shards,
                    "auth1".to_string(),
                    single_authority(),
                )
                .unwrap(),
            );
        }

        Self {
            nodes,
            blocked_routes: HashSet::new(),
            delayed_messages: VecDeque::new(),
        }
    }

    pub fn shard(&self, shard_id: ShardId) -> &ShardedDag {
        self.nodes.get(&shard_id).expect("shard must exist")
    }

    pub fn shard_mut(&mut self, shard_id: ShardId) -> &mut ShardedDag {
        self.nodes.get_mut(&shard_id).expect("shard must exist")
    }

    pub fn isolate_route(&mut self, source: ShardId, target: ShardId) {
        self.blocked_routes.insert((source, target));
    }

    pub fn heal_route(&mut self, source: ShardId, target: ShardId) {
        self.blocked_routes.remove(&(source, target));
    }

    pub fn advance_to_checkpoint(&mut self, shard_id: ShardId) {
        let dag = self.shard_mut(shard_id).local_dag_mut();

        let round1 = dag.create_vertex(vec![], vec![1u8; 32], 1).unwrap();
        dag.add_vertex(round1).unwrap();

        let round2 = dag.create_vertex(vec![], vec![2u8; 32], 2).unwrap();
        dag.add_vertex(round2).unwrap();

        let round3 = dag.create_vertex(vec![], vec![3u8; 32], 3).unwrap();
        dag.add_vertex(round3).unwrap();

        let checkpoint = dag
            .try_commit()
            .unwrap()
            .expect("checkpoint should be created");
        dag.add_checkpoint(checkpoint).unwrap();
    }

    pub fn send_cross_shard(
        &mut self,
        source: ShardId,
        target: ShardId,
        payload: Vec<u8>,
    ) -> CrossShardMessage {
        let routing_key = {
            let source_dag = self.shard(source);
            routing_key_for_target(source_dag, target)
        };

        let dispatch = self
            .shard_mut(source)
            .submit_payload(&routing_key, payload)
            .unwrap();

        let message = match dispatch {
            CrossShardDispatch::Remote(message) => message,
            CrossShardDispatch::Local { .. } => panic!("expected remote dispatch"),
        };

        if self.blocked_routes.contains(&(source, target)) {
            self.delayed_messages.push_back(message.clone());
        } else {
            self.shard_mut(target)
                .receive_message(message.clone())
                .unwrap();
        }

        message
    }

    pub fn flush_delayed(&mut self) -> usize {
        let mut delivered = 0;
        let mut remaining = VecDeque::new();

        while let Some(message) = self.delayed_messages.pop_front() {
            if self
                .blocked_routes
                .contains(&(message.source_shard, message.target_shard))
            {
                remaining.push_back(message);
                continue;
            }

            self.shard_mut(message.target_shard)
                .receive_message(message)
                .unwrap();
            delivered += 1;
        }

        self.delayed_messages = remaining;
        delivered
    }

    pub fn delayed_message_count(&self) -> usize {
        self.delayed_messages.len()
    }

    pub fn drain_inbound(&mut self, shard_id: ShardId) -> Vec<CrossShardMessage> {
        let mut drained = Vec::new();
        while let Some(message) = self.shard_mut(shard_id).pop_inbound() {
            drained.push(message);
        }
        drained
    }
}
