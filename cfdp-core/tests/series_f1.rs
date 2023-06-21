use std::{
    collections::HashMap,
    future::Future,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use camino::Utf8PathBuf;
use cfdp_core::{
    daemon::PutRequest,
    filestore::FileStore,
    pdu::{
        Condition, EntityID, MessageToUser, ProxyOperation, ProxyPutRequest, TransmissionMode,
        UserOperation,
    },
    transport::{PDUTransport, UdpTransport},
};

use rstest::{fixture, rstest};

mod common;
use common::{
    create_daemons, get_filestore, terminate, EntityConstructorReturn, LossyTransport,
    TransportIssue, UsersAndFilestore,
};
use tokio::net::UdpSocket;

#[rstest]
#[timeout(Duration::from_secs(2))]
#[tokio::test]
// Series F1
// Sequence 1 Test
// Test goal:
//  - Establish one-way connectivity
// Configuration:
//  - Unacknowledged
//  - File Size: Small (file fits in single pdu)
async fn f1s1(get_filestore: impl Future<Output = UsersAndFilestore>) {
    let (local_user, _remote_user, filestore) = get_filestore.await;

    let out_file: Utf8PathBuf = "remote/small_f1s1.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    local_user
        .put(PutRequest {
            source_filename: "local/small.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Unacknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while !path_to_out.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }
    assert!(path_to_out.exists());
    tokio::time::sleep(Duration::from_millis(100)).await
}

#[rstest]
#[timeout(Duration::from_secs(5))]
#[tokio::test]
// Series F1
// Sequence 2 Test
// Test goal:
//  - Execute Multiple File Data PDUs
// Configuration:
//  - Unacknowledged
//  - File Size: Medium
async fn f1s2(get_filestore: impl Future<Output = UsersAndFilestore>) {
    let (local_user, _remote_user, filestore) = get_filestore.await;

    let out_file: Utf8PathBuf = "remote/medium_f1s2.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    local_user
        .put(PutRequest {
            source_filename: "local/medium.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Unacknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while !path_to_out.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }
    assert!(path_to_out.exists())
}

#[rstest]
#[timeout(Duration::from_secs(10))]
#[tokio::test]
// Series F1
// Sequence 3 Test
// Test goal:
//  - Execute Two way communication
// Configuration:
//  - Acknowledged
//  - File Size: Medium
async fn f1s3(get_filestore: impl Future<Output = UsersAndFilestore>) {
    let (local_user, _remote_user, filestore) = get_filestore.await;

    let out_file: Utf8PathBuf = "remote/medium_f1s3.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    local_user
        .put(PutRequest {
            source_filename: "local/medium.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while !path_to_out.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }

    assert!(path_to_out.exists())
}

#[fixture]
async fn fixture_f1s4(
    get_filestore: impl Future<Output = UsersAndFilestore>,
    terminate: &Arc<AtomicBool>,
) -> EntityConstructorReturn {
    let (_, _, filestore) = get_filestore.await;
    let remote_udp = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Unable to bind remote UDP.");
    let remote_addr = remote_udp.local_addr().expect("Cannot find local address.");

    let local_udp = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Unable to bind local UDP.");
    let local_addr = local_udp.local_addr().expect("Cannot find local address.");

    let entity_map = {
        let mut temp = HashMap::new();
        temp.insert(EntityID::from(0_u16), local_addr);
        temp.insert(EntityID::from(1_u16), remote_addr);
        temp
    };

    let local_transport =
        LossyTransport::try_from((local_udp, entity_map.clone(), TransportIssue::Rate(13)))
            .expect("Unable to make Lossy Transport.");
    let remote_transport =
        UdpTransport::try_from((remote_udp, entity_map)).expect("Unable to make UdpTransport.");

    let remote_transport_map: HashMap<Vec<EntityID>, Box<dyn PDUTransport + Send>> =
        HashMap::from([(
            vec![EntityID::from(0_u16)],
            Box::new(remote_transport) as Box<dyn PDUTransport + Send>,
        )]);

    let local_transport_map: HashMap<Vec<EntityID>, Box<dyn PDUTransport + Send>> =
        HashMap::from([(
            vec![EntityID::from(1_u16)],
            Box::new(local_transport) as Box<dyn PDUTransport + Send>,
        )]);

    let (local_user, remote_user, local, remote) = create_daemons(
        filestore.clone(),
        local_transport_map,
        remote_transport_map,
        terminate.clone(),
        [None; 3],
    )
    .await;
    (local_user, remote_user, filestore.clone(), local, remote)
}

#[rstest]
#[timeout(Duration::from_secs(30))]
#[tokio::test]
// Series F1
// Sequence 4 Test
// Test goal:
//  - Recovery of Lost data
// Configuration:
//  - Acknowledged
//  - File Size: Medium
//  - ~1% data lost in transport
async fn f1s4(fixture_f1s4: impl Future<Output = EntityConstructorReturn>) {
    let (local_user, _remote_user, filestore, _local, _remote) = fixture_f1s4.await;
    let out_file: Utf8PathBuf = "remote/medium_f1s4.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    local_user
        .put(PutRequest {
            source_filename: "local/medium.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while !path_to_out.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }

    assert!(path_to_out.exists())
}

#[fixture]
async fn fixture_f1s5(
    get_filestore: impl Future<Output = UsersAndFilestore>,
    terminate: &Arc<AtomicBool>,
) -> EntityConstructorReturn {
    let (_, _, filestore) = get_filestore.await;
    let remote_udp = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Unable to bind remote UDP.");
    let remote_addr = remote_udp.local_addr().expect("Cannot find local address.");

    let local_udp = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Unable to bind local UDP.");
    let local_addr = local_udp.local_addr().expect("Cannot find local address.");

    let entity_map = {
        let mut temp = HashMap::new();
        temp.insert(EntityID::from(0_u16), local_addr);
        temp.insert(EntityID::from(1_u16), remote_addr);
        temp
    };

    let local_transport =
        LossyTransport::try_from((local_udp, entity_map.clone(), TransportIssue::Duplicate(13)))
            .expect("Unable to make Lossy Transport.");
    let remote_transport =
        UdpTransport::try_from((remote_udp, entity_map)).expect("Unable to make UdpTransport.");

    let remote_transport_map: HashMap<Vec<EntityID>, Box<dyn PDUTransport + Send>> =
        HashMap::from([(
            vec![EntityID::from(0_u16)],
            Box::new(remote_transport) as Box<dyn PDUTransport + Send>,
        )]);

    let local_transport_map: HashMap<Vec<EntityID>, Box<dyn PDUTransport + Send>> =
        HashMap::from([(
            vec![EntityID::from(1_u16)],
            Box::new(local_transport) as Box<dyn PDUTransport + Send>,
        )]);

    let (local_user, remote_user, local, remote) = create_daemons(
        filestore.clone(),
        local_transport_map,
        remote_transport_map,
        terminate.clone(),
        [None; 3],
    )
    .await;
    (local_user, remote_user, filestore.clone(), local, remote)
}

#[rstest]
#[timeout(Duration::from_secs(30))]
#[tokio::test]
// Series F1
// Sequence 5 Test
// Test goal:
//  - Ignoring duplicated data
// Configuration:
//  - Acknowledged
//  - File Size: Medium
//  - ~1% data duplicated in transport
async fn f1s5(fixture_f1s5: impl Future<Output = EntityConstructorReturn>) {
    let (local_user, _remote_user, filestore, _local, _remote) = fixture_f1s5.await;

    let out_file: Utf8PathBuf = "remote/medium_f1s5.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    local_user
        .put(PutRequest {
            source_filename: "local/medium.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while !path_to_out.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }

    assert!(path_to_out.exists())
}

#[fixture]
async fn fixture_f1s6(
    get_filestore: impl Future<Output = UsersAndFilestore>,
    terminate: &Arc<AtomicBool>,
) -> EntityConstructorReturn {
    let (_, _, filestore) = get_filestore.await;
    let remote_udp = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Unable to bind remote UDP.");
    let remote_addr = remote_udp.local_addr().expect("Cannot find local address.");

    let local_udp = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Unable to bind local UDP.");
    let local_addr = local_udp.local_addr().expect("Cannot find local address.");

    let entity_map = {
        let mut temp = HashMap::new();
        temp.insert(EntityID::from(0_u16), local_addr);
        temp.insert(EntityID::from(1_u16), remote_addr);
        temp
    };

    let local_transport =
        LossyTransport::try_from((local_udp, entity_map.clone(), TransportIssue::Reorder(13)))
            .expect("Unable to make Lossy Transport.");
    let remote_transport =
        UdpTransport::try_from((remote_udp, entity_map)).expect("Unable to make UdpTransport.");

    let remote_transport_map: HashMap<Vec<EntityID>, Box<dyn PDUTransport + Send>> =
        HashMap::from([(
            vec![EntityID::from(0_u16)],
            Box::new(remote_transport) as Box<dyn PDUTransport + Send>,
        )]);

    let local_transport_map: HashMap<Vec<EntityID>, Box<dyn PDUTransport + Send>> =
        HashMap::from([(
            vec![EntityID::from(1_u16)],
            Box::new(local_transport) as Box<dyn PDUTransport + Send>,
        )]);

    let (local_user, remote_user, local, remote) = create_daemons(
        filestore.clone(),
        local_transport_map,
        remote_transport_map,
        terminate.clone(),
        [None; 3],
    )
    .await;
    (local_user, remote_user, filestore.clone(), local, remote)
}

#[rstest]
#[timeout(Duration::from_secs(5))]
#[tokio::test]
// Series F1
// Sequence 6 Test
// Test goal:
//  - Reorder Data test
// Configuration:
//  - Acknowledged
//  - File Size: Medium
//  - ~1% data re-ordered in transport
async fn f1s6(fixture_f1s6: impl Future<Output = EntityConstructorReturn>) {
    // let mut user = User::new(Some(_local_path))
    let (local_user, _remote_user, filestore, _local, _remote) = fixture_f1s6.await;

    let out_file: Utf8PathBuf = "remote/medium_f1s6.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    local_user
        .put(PutRequest {
            source_filename: "local/medium.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while !path_to_out.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }

    assert!(path_to_out.exists())
}

#[rstest]
#[timeout(Duration::from_secs(10))]
#[tokio::test]
// Series F1
// Sequence 7 Test
// Test goal:
//  - Check User application messaging
// Configuration:
//  - Acknowledged
//  - File Size: Zero
//  - Have proxy put request send Entity 0 data,
//  -   then have a proxy put request in THAT message send data back to entity 1
async fn f1s7(get_filestore: impl Future<Output = UsersAndFilestore>) {
    let (local_user, _remote_user, filestore) = get_filestore.await;

    let out_file: Utf8PathBuf = "/remote/medium_f1s7.txt".into();
    let interim_file: Utf8PathBuf = "/local/medium_f1s7.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);
    let path_interim = filestore.get_native_path(&interim_file);
    println!("f1s7 step 20");
    local_user
        .put(PutRequest {
            source_filename: "".into(),
            destination_filename: "".into(),
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![
                MessageToUser::from(UserOperation::ProxyOperation(
                    ProxyOperation::ProxyPutRequest(ProxyPutRequest {
                        destination_entity_id: EntityID::from(0_u16),
                        source_filename: "/local/medium.txt".into(),
                        destination_filename: interim_file.clone(),
                    }),
                )),
                MessageToUser::from(UserOperation::ProxyOperation(
                    ProxyOperation::ProxyTransmissionMode(TransmissionMode::Acknowledged),
                )),
            ],
        })
        .await
        .expect("unable to send put request.");
    println!("f1s7 step 30");
    while !path_interim.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }
    assert!(path_interim.exists());
    println!("f1s7 step 100");

    local_user
        .put(PutRequest {
            source_filename: "".into(),
            destination_filename: "".into(),
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![
                MessageToUser::from(UserOperation::ProxyOperation(
                    ProxyOperation::ProxyPutRequest(ProxyPutRequest {
                        destination_entity_id: EntityID::from(0_u16),
                        source_filename: "".into(),
                        destination_filename: "".into(),
                    }),
                )),
                MessageToUser::from(UserOperation::ProxyOperation(
                    ProxyOperation::ProxyMessageToUser(MessageToUser::from(
                        UserOperation::ProxyOperation(ProxyOperation::ProxyPutRequest(
                            ProxyPutRequest {
                                destination_entity_id: EntityID::from(1_u16),
                                source_filename: interim_file,
                                destination_filename: out_file,
                            },
                        )),
                    )),
                )),
                MessageToUser::from(UserOperation::ProxyOperation(
                    ProxyOperation::ProxyMessageToUser(MessageToUser::from(
                        UserOperation::ProxyOperation(ProxyOperation::ProxyTransmissionMode(
                            TransmissionMode::Acknowledged,
                        )),
                    )),
                )),
            ],
        })
        .await
        .expect("Unable to send put request.");

    println!("f1s7 step 200");

    while !path_to_out.exists() {
        tokio::time::sleep(Duration::from_millis(100)).await
    }

    assert!(path_to_out.exists())
}

#[rstest]
#[timeout(Duration::from_secs(10))]
#[tokio::test]
// Series F1
// Sequence 8 Test
// Test goal:
//  - Check User Cancel Functionality
// Configuration:
//  - Cancel initiated from Sender
async fn f1s8(get_filestore: impl Future<Output = UsersAndFilestore>) {
    let (local_user, _remote_user, filestore) = get_filestore.await;

    let out_file: Utf8PathBuf = "remote/medium_f1s8.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    let id = local_user
        .put(PutRequest {
            source_filename: "local/medium.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while local_user
        .report(id)
        .await
        .expect("cannot send report")
        .is_none()
    {
        tokio::time::sleep(Duration::from_millis(100)).await
    }
    local_user.cancel(id).await.expect("unable to cancel.");

    let mut report = local_user
        .report(id)
        .await
        .expect("Unable to send Report Request.")
        .unwrap();

    while report.condition != Condition::CancelReceived {
        tokio::time::sleep(Duration::from_millis(100)).await;
        report = local_user
            .report(id)
            .await
            .expect("Unable to send Report Request.")
            .unwrap();
    }

    assert_eq!(report.condition, Condition::CancelReceived);

    assert!(!path_to_out.exists())
}

#[rstest]
#[timeout(Duration::from_secs(30))]
#[tokio::test]
// Series F1
// Sequence 9 Test
// Test goal:
//  - Check User Cancel Functionality
// Configuration:
//  - Cancel initiated from Receiver
async fn f1s9(get_filestore: impl Future<Output = UsersAndFilestore>) {
    let _ = env_logger::builder().is_test(true).try_init();

    let (local_user, remote_user, filestore) = get_filestore.await;

    let out_file: Utf8PathBuf = "remote/medium_f1s9.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    let id = local_user
        .put(PutRequest {
            source_filename: "local/medium.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Acknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");
    while remote_user
        .report(id)
        .await
        .expect("cannot send report")
        .is_none()
    {
        tokio::time::sleep(Duration::from_micros(1)).await
    }
    remote_user.cancel(id).await.expect("unable to cancel.");

    let mut report = remote_user
        .report(id)
        .await
        .expect("Unable to send Report Request.")
        .unwrap();

    while report.condition != Condition::CancelReceived {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        report = remote_user
            .report(id)
            .await
            .expect("Unable to send Report Request.")
            .unwrap();
        println!("report: {:?}", report);
    }

    assert_eq!(report.condition, Condition::CancelReceived);

    assert!(!path_to_out.exists())
}

#[rstest]
#[timeout(Duration::from_secs(10))]
#[tokio::test]
// Series F1
// Sequence 10 Test
// Test goal:
//  - Check User Cancel Functionality
// Configuration:
//  - Unacknowledged Transmission
//  - Cancel initiated from Sender
async fn f1s10(get_filestore: impl Future<Output = UsersAndFilestore>) {
    let (local_user, _remote_user, filestore) = get_filestore.await;

    let out_file: Utf8PathBuf = "remote/large_f1s10.txt".into();
    let path_to_out = filestore.get_native_path(&out_file);

    let id = local_user
        .put(PutRequest {
            source_filename: "local/large.txt".into(),
            destination_filename: out_file,
            destination_entity_id: EntityID::from(1_u16),
            transmission_mode: TransmissionMode::Unacknowledged,
            filestore_requests: vec![],
            message_to_user: vec![],
        })
        .await
        .expect("unable to send put request.");

    while local_user
        .report(id)
        .await
        .expect("cannot send report")
        .is_none()
    {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    local_user.cancel(id).await.expect("unable to cancel.");

    let mut report = local_user
        .report(id)
        .await
        .expect("Unable to send Report Request.")
        .unwrap();

    while report.condition != Condition::CancelReceived {
        tokio::time::sleep(Duration::from_millis(100)).await;
        report = local_user
            .report(id)
            .await
            .expect("Unable to send Report Request.")
            .unwrap();
    }

    assert_eq!(report.condition, Condition::CancelReceived);

    assert!(!path_to_out.exists())
}
