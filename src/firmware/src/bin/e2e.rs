use core::ffi::CStr;

use drivers::testing::esp_https_one_authority::EspHttpsClientTrustingOneAuthority;
use drivers::udp_socket::StdUdpClient;
use hal::Fault;
use infrastructure::own_directory::{CardDirectory, OwnDirectory};
use infrastructure::testing::to_local_stubs::{RedirectedHttp, RedirectedUdp};

use firmware::app_image;
use firmware::board::Board;

firmware::image_description!();

const OWN_DIRECTORY: CardDirectory = CardDirectory::new("cute-display-e2e");
/// Made by `tools/e2e`'s `cute_display_e2e.authority`.
const AUTHORITY_PEM: &str = concat!(include_str!("../../../../tools/e2e/device/ca/ca.pem"), "\0");

fn main() -> Result<(), Fault> {
    app_image::start_logging("Cute Display test image");
    let board = Board::bring_up()?;
    let authority = CStr::from_bytes_with_nul(AUTHORITY_PEM.as_bytes()).map_err(Fault::new)?;
    let own_directory_on_card = || board.sd_card.clone().map(|card| OwnDirectory::new(card, OWN_DIRECTORY));
    let https = RedirectedHttp::new(EspHttpsClientTrustingOneAuthority::new(authority), own_directory_on_card());
    let udp = RedirectedUdp::new(StdUdpClient, own_directory_on_card());
    match app_image::run(board, OWN_DIRECTORY, https, udp)? {}
}
