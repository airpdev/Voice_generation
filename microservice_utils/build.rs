fn main() {
    let auth = "./proto/auth_service.proto";
    let user = "./proto/user_service.proto";
    let workspace = "./proto/workspace_service.proto";
    let address_book = "./proto/address_book_service.proto";
    let ai_studio = "./proto/ai_studio_service.proto";
    let file = "./proto/file_manager_service.proto";

    tonic_build::configure()
        .build_server(true)
        .compile(&[auth, workspace, user, address_book, ai_studio, file], &["."])
        .unwrap_or_else(|e| panic!("protobuf compile error: {}", e));
}