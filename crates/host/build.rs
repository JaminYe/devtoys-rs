fn sanitize_version(input: &str) -> Result<String, String> {
    let s = input.trim();
    let s = s
        .strip_prefix('v')
        .or_else(|| s.strip_prefix('V'))
        .unwrap_or(s);
    if s.is_empty() {
        return Err("version string is empty".into());
    }

    // Split off build metadata (+)
    let (ver_pre, build) = match s.split_once('+') {
        Some((v, b)) => (v, Some(b)),
        None => (s, None),
    };

    // Split off prerelease (-)
    let (core, pre) = match ver_pre.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (ver_pre, None),
    };

    // Validate core: MAJOR.MINOR.PATCH
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return Err(format!(
            "core version must be MAJOR.MINOR.PATCH, got '{core}'"
        ));
    }

    for (idx, part) in parts.iter().enumerate() {
        let name = match idx {
            0 => "MAJOR",
            1 => "MINOR",
            _ => "PATCH",
        };
        if part.is_empty() {
            return Err(format!("{name} version part is empty"));
        }
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("{name} version part '{part}' contains non-digits"));
        }
        if part.len() > 1 && part.starts_with('0') {
            return Err(format!("{name} version part '{part}' has leading zero"));
        }
        if part.parse::<u64>().is_err() {
            return Err(format!(
                "{name} version part '{part}' exceeds integer limit"
            ));
        }
    }

    // Validate prerelease identifiers
    if let Some(p) = pre {
        if p.is_empty() {
            return Err("prerelease identifier cannot be empty".into());
        }
        for ident in p.split('.') {
            if ident.is_empty() {
                return Err("prerelease dot-separated identifier cannot be empty".into());
            }
            if !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(format!(
                    "prerelease identifier '{ident}' contains invalid characters"
                ));
            }
            if ident.chars().all(|c| c.is_ascii_digit())
                && ident.len() > 1
                && ident.starts_with('0')
            {
                return Err(format!(
                    "numeric prerelease identifier '{ident}' cannot have leading zero"
                ));
            }
        }
    }

    // Validate build identifiers
    if let Some(b) = build {
        if b.is_empty() {
            return Err("build metadata identifier cannot be empty".into());
        }
        for ident in b.split('.') {
            if ident.is_empty() {
                return Err("build metadata dot-separated identifier cannot be empty".into());
            }
            if !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(format!(
                    "build metadata identifier '{ident}' contains invalid characters"
                ));
            }
        }
    }

    Ok(s.to_string())
}

fn main() {
    println!("cargo:rerun-if-env-changed=DEVTOYS_APP_VERSION");
    let version = match std::env::var("DEVTOYS_APP_VERSION") {
        Ok(val) if !val.trim().is_empty() => match sanitize_version(&val) {
            Ok(v) => v,
            Err(e) => panic!("Invalid DEVTOYS_APP_VERSION '{val}': {e}"),
        },
        _ => env!("CARGO_PKG_VERSION").to_string(),
    };
    println!("cargo:rustc-env=DEVTOYS_APP_VERSION={version}");

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let _ = embed_resource::compile("assets/devtoys.rc", embed_resource::NONE);
    }
}
