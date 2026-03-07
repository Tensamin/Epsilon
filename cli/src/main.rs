pub mod bin;
pub mod ui;

#[tokio::main]
async fn main() {
    // 99.9 % Vibe Coded
    bin::run_binary_to_comm_converter();
    ui::run_comm_to_binary_converter();
}
