#[macro_export]
macro_rules! role {
    ($x: expr, $y: expr) => {
        use std::str::FromStr;

        match TokenRole::from_str($y) {
            Ok(TokenRole::Guest) => {
                if $x != TokenRole::Guest {
                    let ret = serde_json::json!({
                        "error": format!("{}", "Access Denied"),
                    });
                    return Err(into_response(403, ret));
                }                    
            }
            Ok(TokenRole::Admin) => {}
            Ok(TokenRole::User) | Ok(TokenRole::DefaultTokenRole) => {
                if $x == TokenRole::Admin {
                    let ret = serde_json::json!({
                        "error": format!("{}", "Access Denied"),
                    });
                    return Err(into_response(403, ret));
                }   
            }
            Err(_) => {
                let ret = serde_json::json!({
                    "error": format!("{}", "Token role not found"),
                });
                return Err(into_response(404, ret));
            }
        }
    }
}
