use coral_rs::api::generated::types::{
    AgentGraphRequest, AgentOptionValue, AgentRegistryIdentifier, GraphAgentProvider,
    GraphAgentRequest, RuntimeId, SessionRequest,
};
use coral_rs::api::generated::{Client, Error};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let agent = GraphAgentRequest {
        blocking: None,
        coral_plugins: vec![],
        custom_tool_access: vec![],
        description: None,
        id: AgentRegistryIdentifier {
            name: "example-options".to_string(),
            version: "0.0.1".to_string(),
        },
        name: "example-options".to_string(),
        options: HashMap::from([
            ("REQUIRED_I8".to_string(), AgentOptionValue::I8(42)),
            ("REQUIRED_I16".to_string(), AgentOptionValue::I16(-1234)),
            ("REQUIRED_I32".to_string(), AgentOptionValue::I32(987654)),
            (
                "REQUIRED_I64".to_string(),
                AgentOptionValue::I64(-9876543210),
            ),
            ("REQUIRED_U8".to_string(), AgentOptionValue::U8(200)),
            ("REQUIRED_U16".to_string(), AgentOptionValue::U16(50000)),
            (
                "REQUIRED_U32".to_string(),
                AgentOptionValue::U32(3000000000),
            ),
            (
                "REQUIRED_U64".to_string(),
                AgentOptionValue::U64("18446744073709551615".to_string()),
            ),
            (
                "REQUIRED_F32".to_string(),
                AgentOptionValue::F32(std::f32::consts::PI),
            ),
            (
                "REQUIRED_F64".to_string(),
                AgentOptionValue::F64(std::f64::consts::E),
            ),
            ("REQUIRED_BOOL".to_string(), AgentOptionValue::Bool(true)),
            (
                "REQUIRED_STRING".to_string(),
                AgentOptionValue::String("Hello, World!".to_string()),
            ),
            (
                "REQUIRED_BLOB".to_string(),
                AgentOptionValue::Blob(vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]),
            ),
            ("REQUIRED_NUMBER".to_string(), AgentOptionValue::F64(12.12)),
            (
                "REQUIRED_SECRET".to_string(),
                AgentOptionValue::String("my-secret-key-12345".to_string()),
            ),
            (
                "FS_LIST_I8".to_string(),
                AgentOptionValue::ListI8(vec![-128, 0, 42, 127]),
            ),
            (
                "FS_LIST_I16".to_string(),
                AgentOptionValue::ListI16(vec![-32768, -1000, 0, 1000, 32767]),
            ),
            (
                "FS_LIST_I32".to_string(),
                AgentOptionValue::ListI32(vec![-2147483648, -12345, 0, 987654, 2147483647]),
            ),
            (
                "FS_LIST_I64".to_string(),
                AgentOptionValue::ListI64(vec![
                    -9223372036854775808,
                    -9876543210,
                    0,
                    1234567890,
                    9223372036854775807,
                ]),
            ),
            (
                "FS_LIST_U8".to_string(),
                AgentOptionValue::ListU8(vec![0, 42, 128, 200, 255]),
            ),
            (
                "FS_LIST_U16".to_string(),
                AgentOptionValue::ListU16(vec![0, 1000, 32768, 50000, 65535]),
            ),
            (
                "FS_LIST_U32".to_string(),
                AgentOptionValue::ListU32(vec![0, 100000, 2147483648, 3000000000, 4294967295]),
            ),
            (
                "FS_LIST_U64".to_string(),
                AgentOptionValue::ListU64(vec![
                    "0".to_string(),
                    "1000000000".to_string(),
                    "9223372036854775808".to_string(),
                    "18446744073709551615".to_string(),
                ]),
            ),
            (
                "FS_LIST_F32".to_string(),
                AgentOptionValue::ListF32(vec![-3.14, 0.0, 1.618, 2.718, std::f32::consts::PI]),
            ),
            (
                "FS_LIST_F64".to_string(),
                AgentOptionValue::ListF64(vec![
                    -std::f64::consts::E,
                    0.0,
                    std::f64::consts::SQRT_2,
                    std::f64::consts::E,
                    std::f64::consts::PI,
                ]),
            ),
            (
                "FS_LIST_STRING".to_string(),
                AgentOptionValue::ListString(vec![
                    "hello".to_string(),
                    "world".to_string(),
                    "foo".to_string(),
                    "bar".to_string(),
                ]),
            ),
            (
                "FS_LIST_BLOB".to_string(),
                AgentOptionValue::ListBlob(vec![
                    vec![0x48, 0x65, 0x6C, 0x6C, 0x6F],
                    vec![0x57, 0x6F, 0x72, 0x6C, 0x64],
                    vec![0x01, 0x02, 0x03],
                ]),
            ),
        ]),
        provider: GraphAgentProvider::Local {
            runtime: RuntimeId::Executable,
        },
        system_prompt: None,
    };

    match Client::new("http://localhost:5555")
        .create_session(&SessionRequest {
            agent_graph_request: AgentGraphRequest {
                agents: vec![agent],
                custom_tools: Default::default(),
                groups: vec![vec!["example-options".to_string()]],
            },
            application_id: "".to_string(),
            privacy_key: "".to_string(),
            session_id: None,
        })
        .await
    {
        Ok(a) => println!("session id: {}", a.session_id),
        Err(Error::ErrorResponse(e)) => {
            eprintln!("{e:#?}")
        }
        Err(e) => panic!("unexpected error: {}", e),
    }
}
