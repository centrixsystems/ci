use dagger_sdk::{Directory, Query};

/// Deploy to dev server via SSH (rsync + build + restart).
/// Requires SSHPASS environment variable.
pub async fn run(client: &Query, source: Directory, host: &str) -> eyre::Result<String> {
    let password = std::env::var("SSHPASS").unwrap_or_default();
    if password.is_empty() {
        return Err(eyre::eyre!("SSHPASS environment variable not set"));
    }

    let ssh_password = client.set_secret("ssh-password", password);

    let script = format!(
        r#"
set -e

SSH_OPTS="-o StrictHostKeyChecking=no -o PreferredAuthentications=password -o PubkeyAuthentication=no"

echo "[1/4] Syncing source..."
rsync -az --delete \
    --exclude='.git' --exclude='target' --exclude='node_modules' \
    -e "sshpass -e ssh $SSH_OPTS -p 22" \
    /deploy/source/ ubuntu-server@{host}:rust-erp-dev/rust-erp/

echo "[2/4] Building on server..."
sshpass -e ssh $SSH_OPTS -p 22 ubuntu-server@{host} \
    'source $HOME/.cargo/env && cd $HOME/rust-erp-dev/rust-erp && cargo build --release 2>&1 | tail -5'

echo "[3/4] Deploying binary + static..."
sshpass -e ssh $SSH_OPTS -p 22 ubuntu-server@{host} \
    'echo $SSHPASS | sudo -S systemctl stop erp.service && \
     echo $SSHPASS | sudo -S cp $HOME/rust-erp-dev/rust-erp/target/release/erp-server /opt/rust-erp/erp-server && \
     echo $SSHPASS | sudo -S cp -r $HOME/rust-erp-dev/rust-erp/erp_web/static/* /opt/rust-erp/erp_web/static/ && \
     echo $SSHPASS | sudo -S systemctl start erp.service'

echo "[4/4] Health check..."
sleep 3
curl -sf http://{host}:9089/health || echo "Warning: health check failed"
echo "Deploy complete."
"#,
        host = host
    );

    let output = client
        .container()
        .from("debian:bookworm-slim")
        .with_exec(vec!["apt-get", "update"])
        .with_exec(vec![
            "apt-get", "install", "-y",
            "sshpass", "rsync", "openssh-client", "curl",
        ])
        .with_secret_variable("SSHPASS", ssh_password)
        .with_workdir("/deploy")
        .with_directory("/deploy/source", source)
        .with_exec(vec!["sh", "-c", script.as_str()])
        .stdout()
        .await?;

    Ok(format!("[deploy] {output}"))
}
