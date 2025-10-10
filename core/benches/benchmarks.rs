//! Performance benchmarks for RustIRCd core

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rustircd_core::*;
use std::time::Duration;
use uuid::Uuid;

fn benchmark_message_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_parsing");
    
    let messages = vec![
        "NICK alice",
        "USER alice 0 * :Alice Wonderland",
        ":alice!user@host PRIVMSG #channel :Hello world",
        ":server.example.com 001 alice :Welcome to the Internet Relay Network",
        "JOIN #channel",
        "PART #channel :Goodbye",
        "QUIT :Leaving",
    ];
    
    for msg in messages {
        group.bench_with_input(BenchmarkId::from_parameter(msg), msg, |b, msg| {
            b.iter(|| Message::parse(black_box(msg)))
        });
    }
    
    group.finish();
}

fn benchmark_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");
    
    let messages = vec![
        Message::new(MessageType::Nick, vec!["alice".to_string()]),
        Message::new(MessageType::Join, vec!["#channel".to_string()]),
        Message::with_prefix(
            Prefix::User {
                nick: "alice".to_string(),
                user: "user".to_string(),
                host: "host".to_string(),
            },
            MessageType::PrivMsg,
            vec!["#channel".to_string(), "Hello world".to_string()],
        ),
    ];
    
    for (i, msg) in messages.iter().enumerate() {
        group.bench_with_input(BenchmarkId::from_parameter(i), msg, |b, msg| {
            b.iter(|| msg.to_string())
        });
    }
    
    group.finish();
}

fn benchmark_database_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("database");
    
    // Benchmark user insertion
    group.bench_function("add_user", |b| {
        let db = Database::new(10000, 30);
        let mut counter = 0;
        b.iter(|| {
            let user = User::new(
                format!("user{}", counter),
                "username".to_string(),
                "Real Name".to_string(),
                "host.example.com".to_string(),
                "server.example.com".to_string(),
            );
            counter += 1;
            db.add_user(black_box(user))
        });
    });
    
    // Benchmark user lookup by nickname
    group.bench_function("get_user_by_nick", |b| {
        let db = Database::new(10000, 30);
        for i in 0..1000 {
            let user = User::new(
                format!("user{}", i),
                "username".to_string(),
                "Real Name".to_string(),
                "host.example.com".to_string(),
                "server.example.com".to_string(),
            );
            db.add_user(user).ok();
        }
        
        b.iter(|| {
            db.get_user_by_nick(black_box("user500"))
        });
    });
    
    // Benchmark user update
    group.bench_function("update_user", |b| {
        let db = Database::new(10000, 30);
        let mut user = User::new(
            "testuser".to_string(),
            "username".to_string(),
            "Real Name".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        db.add_user(user.clone()).ok();
        
        b.iter(|| {
            user.nick = format!("testuser{}", rand::random::<u32>());
            db.update_user(black_box(user.clone()))
        });
    });
    
    group.finish();
}

fn benchmark_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");
    
    // Benchmark LRU cache
    group.bench_function("lru_cache_insert", |b| {
        let cache = LruCache::<String, String>::new(1000, Duration::from_secs(60));
        let mut counter = 0;
        b.iter(|| {
            cache.insert(
                black_box(format!("key{}", counter)),
                black_box(format!("value{}", counter))
            );
            counter += 1;
        });
    });
    
    group.bench_function("lru_cache_get", |b| {
        let cache = LruCache::<String, String>::new(1000, Duration::from_secs(60));
        for i in 0..1000 {
            cache.insert(format!("key{}", i), format!("value{}", i));
        }
        
        b.iter(|| {
            cache.get(black_box(&"key500".to_string()))
        });
    });
    
    // Benchmark message cache
    group.bench_function("message_cache_insert", |b| {
        let cache = MessageCache::new(1000, Duration::from_secs(60));
        let mut counter = 0;
        b.iter(|| {
            cache.insert(
                black_box(format!("PING :server{}", counter)),
                black_box(format!("PONG :server{}\r\n", counter))
            );
            counter += 1;
        });
    });
    
    group.bench_function("message_cache_get", |b| {
        let cache = MessageCache::new(1000, Duration::from_secs(60));
        for i in 0..1000 {
            cache.insert(
                format!("PING :server{}", i),
                format!("PONG :server{}\r\n", i)
            );
        }
        
        b.iter(|| {
            cache.get(black_box("PING :server500"))
        });
    });
    
    group.finish();
}

fn benchmark_broadcast_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("broadcast");
    
    group.bench_function("subscribe_to_channel", |b| {
        let system = BroadcastSystem::new();
        let mut counter = 0;
        b.iter(|| {
            let user_id = Uuid::new_v4();
            system.subscribe_to_channel(
                black_box(user_id),
                black_box(format!("#channel{}", counter % 100))
            );
            counter += 1;
        });
    });
    
    group.bench_function("unsubscribe_from_channel", |b| {
        let system = BroadcastSystem::new();
        let user_ids: Vec<_> = (0..1000).map(|_| Uuid::new_v4()).collect();
        for id in &user_ids {
            system.subscribe_to_channel(*id, "#test".to_string());
        }
        
        let mut idx = 0;
        b.iter(|| {
            system.unsubscribe_from_channel(
                black_box(&user_ids[idx % user_ids.len()]),
                black_box("#test")
            );
            idx += 1;
        });
    });
    
    group.finish();
}

fn benchmark_batch_optimizer(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("batch_optimizer");
    
    group.bench_function("add_message", |b| {
        let config = BatchConfig::default();
        let optimizer = BatchOptimizer::new(config);
        let target_id = Uuid::new_v4();
        
        b.to_async(&rt).iter(|| async {
            let msg = Message::new(
                MessageType::PrivMsg,
                vec!["#test".to_string(), "Hello".to_string()]
            );
            optimizer.add_message(black_box(target_id), black_box(msg)).await
        });
    });
    
    group.bench_function("get_ready_batches", |b| {
        let config = BatchConfig {
            max_batch_size: 10,
            max_batch_delay: Duration::from_millis(10),
            max_batch_bytes: 1000,
        };
        let optimizer = BatchOptimizer::new(config);
        
        b.to_async(&rt).iter(|| async {
            optimizer.get_ready_batches().await
        });
    });
    
    group.finish();
}

fn benchmark_validation(c: &mut Criterion) {
    use rustircd_core::utils::string;
    
    let mut group = c.benchmark_group("validation");
    
    group.bench_function("validate_nickname", |b| {
        b.iter(|| {
            string::is_valid_nickname(black_box("alice123"), black_box(9))
        });
    });
    
    group.bench_function("validate_channel_name", |b| {
        b.iter(|| {
            string::is_valid_channel_name(black_box("#channel"))
        });
    });
    
    group.bench_function("validate_username", |b| {
        b.iter(|| {
            string::is_valid_username(black_box("username"))
        });
    });
    
    group.finish();
}

fn benchmark_user_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("user_modes");
    
    group.bench_function("set_mode", |b| {
        let mut user = User::new(
            "alice".to_string(),
            "user".to_string(),
            "Alice User".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        
        b.iter(|| {
            user.set_mode(black_box(UserMode::Invisible), black_box(true));
        });
    });
    
    group.bench_function("has_mode", |b| {
        let mut user = User::new(
            "alice".to_string(),
            "user".to_string(),
            "Alice User".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        user.set_mode(UserMode::Invisible, true);
        
        b.iter(|| {
            user.has_mode(black_box(&UserMode::Invisible))
        });
    });
    
    group.finish();
}

fn benchmark_netsplit_operations(c: &mut Criterion) {
    use rustircd_core::server_connection::ReconnectionState;
    
    let mut group = c.benchmark_group("netsplit");
    
    // Benchmark reconnection state calculations
    group.bench_function("calculate_backoff", |b| {
        let mut state = ReconnectionState::new();
        b.iter(|| {
            state.attempts = black_box(3);
            state.calculate_next_delay(30, 1800)
        });
    });
    
    group.bench_function("should_attempt_reconnect", |b| {
        let state = ReconnectionState::new();
        b.iter(|| {
            state.should_attempt_reconnect()
        });
    });
    
    // Benchmark user state transitions
    group.bench_function("user_state_check", |b| {
        let user = User::new(
            "alice".to_string(),
            "user".to_string(),
            "Alice User".to_string(),
            "host.example.com".to_string(),
            "server.example.com".to_string(),
        );
        
        b.iter(|| {
            black_box(user.state == rustircd_core::UserState::NetSplit)
        });
    });
    
    group.finish();
}

fn benchmark_server_to_server(c: &mut Criterion) {
    let mut group = c.benchmark_group("server_to_server");
    
    // Benchmark message construction for server propagation
    group.bench_function("create_kill_message", |b| {
        b.iter(|| {
            Message::with_prefix(
                Prefix::Server(black_box("server.example.com".to_string())),
                MessageType::Kill,
                vec![
                    black_box("victim".to_string()),
                    black_box("Killed by operator".to_string())
                ]
            )
        });
    });
    
    group.bench_function("create_server_quit", |b| {
        b.iter(|| {
            Message::with_prefix(
                Prefix::Server(black_box("hub.example.com".to_string())),
                MessageType::ServerQuit,
                vec![
                    black_box("leaf.example.com".to_string()),
                    black_box("Connection timeout".to_string())
                ]
            )
        });
    });
    
    // Benchmark nick propagation message
    group.bench_function("create_nick_propagation", |b| {
        b.iter(|| {
            Message::with_prefix(
                Prefix::User {
                    nick: black_box("oldnick".to_string()),
                    user: black_box("user".to_string()),
                    host: black_box("host.example.com".to_string()),
                },
                MessageType::Nick,
                vec![black_box("newnick".to_string())]
            )
        });
    });
    
    group.finish();
}

fn benchmark_network_topology(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_topology");
    
    // Benchmark split severity calculation
    group.bench_function("calculate_split_severity", |b| {
        b.iter(|| {
            let connected = black_box(7);
            let total = black_box(10);
            let percentage = (connected as f64 / total as f64) * 100.0;
            
            if percentage >= 75.0 {
                "Minor"
            } else if percentage >= 50.0 {
                "Major"
            } else {
                "Critical"
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_message_parsing,
    benchmark_message_serialization,
    benchmark_database_operations,
    benchmark_cache_operations,
    benchmark_broadcast_operations,
    benchmark_batch_optimizer,
    benchmark_validation,
    benchmark_user_modes,
    benchmark_netsplit_operations,
    benchmark_server_to_server,
    benchmark_network_topology
);

criterion_main!(benches);






