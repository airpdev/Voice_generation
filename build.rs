fn main() {
    let auth = "./proto/auth_service.proto";

    tonic_build::configure()
        .build_server(true)
        .compile(&[auth], &["."])
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));
}