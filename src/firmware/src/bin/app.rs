use app::image::OWN_DIRECTORY;
use drivers::esp_http::EspHttpsClient;
use drivers::udp_socket::StdUdpClient;
use hal::Fault;

use firmware::app_image;
use firmware::board::Board;

firmware::image_description!();

fn main() -> Result<(), Fault> {
    app_image::start_logging("Cute Display");
    let board = Board::bring_up()?;
    match app_image::run(board, OWN_DIRECTORY, EspHttpsClient, StdUdpClient)? {}
}
