# Tests end-to-end sur cible : image `e2e`, stubs réseau sur le LAN

## Avancement

- [x] 1. Règles et design
- [x] 2. Le dossier de l'image sur la carte, configurable pour toutes les images
- [x] 2 bis. Redirection du réseau de l'image `e2e`
- [x] 3. Driver HTTP : choisir à qui faire confiance
- [x] 4. Image `e2e`
- [x] 5. Serveur de stubs
- [x] 6. Harnais pytest sur cible
- [ ] 7. Premiers scénarios marqués `device`
- [ ] 8. Docs

## À reprendre plus tard

- **Le contrôle des chemins existe deux fois** : `OwnDirectory::inside` et
  `SdPath::parse` (console) vérifient la même chose. Un type de chemin relatif dans
  `hal::storage` pourrait servir aux deux.

## Contexte

Les scénarios de `tools/e2e/` pilotent le simulateur par la console. La ligne
« End-to-end, device » de `docs/testing/README.md` est « Not yet ». On veut lancer une
partie de ces scénarios (les rapides) sur le vrai Habity, pour couvrir ce que seule la
cible a : mémoire, piles, TLS, carte, RTC, speaker, temps réel. Le tout sans abîmer la
prod.

Décisions prises :
- **Image dédiée `e2e`**, pas de mode de test dans l'image de prod.
- **Sur la carte**, chaque image lit et écrit dans un dossier qui lui est propre, donné
  par sa composition root : `cute-display/` pour la prod, `cute-display-e2e/` pour
  l'image de test. La règle 6 est
  mise à jour en conséquence. Côté NVS, rien pour l'instant ; la règle 7 réserve un
  namespace e2e.
- **Réseau** : HTTPS vers un serveur de stubs sur le PC, avec une CA de test. NTP
  redirigé vers un stub UDP. L'IP du PC est dans une conf sur la carte, posée avant les
  tests.
- **RTC** : pas remis à l'heure après les tests.

## Comment ESP-IDF fait confiance à la CA de test

- `EspHttpsClient` (`src/engine/drivers/src/esp_http.rs`) passe aujourd'hui
  `crt_bundle_attach: esp_crt_bundle_attach`. mbedTLS vérifie alors le serveur contre le
  bundle de CA publiques d'ESP-IDF.
- `esp_idf_svc::http::client::Configuration` a aussi `server_certificate:
  Option<X509<'static>>`. Il est copié dans `esp_http_client_config_t.cert_pem`. Malgré
  son nom, c'est le **certificat de CA** contre lequel esp-tls vérifie la chaîne du
  serveur (`mbedtls_ssl_conf_ca_chain` en `VERIFY_REQUIRED`). Le nom d'hôte de l'URL est
  aussi vérifié (`mbedtls_ssl_set_hostname`).
- L'image `e2e` met `server_certificate = X509::pem(<CA de test>)` et **pas** de
  `crt_bundle_attach`. Elle ne fait donc confiance qu'à la CA de test : elle ne peut
  joindre en HTTPS que le serveur de stubs, jamais le vrai Internet. C'est voulu, et c'est
  une sécurité de plus.
- Le serveur de stubs présente un certificat signé par la CA, émis à chaque session pour
  l'IP du PC. On met cette IP dans le CN et en SAN `dNSName` (mbedTLS compare la chaîne
  de l'hôte telle quelle), et aussi en SAN `iPAddress`.
- Les dates de validité sont larges (de 2020 à 2099). Les scénarios règlent l'horloge à
  `AT_THE_RECORDING`, qui peut tomber avant la date d'émission du certificat.
- La CA (certificat et clé) est commitée dans `tools/e2e/device/ca/`. Ainsi `just lint`
  et le build de `e2e` ne dépendent d'aucun fichier généré. Le risque est faible : seule
  l'image de test la croit.

## Étapes

### 1. Règles et design (en premier, règle 2)

- **`AGENTS.md`, règle 6** : la prod écrit sous `cute-display/`, l'image de test sous
  `cute-display-e2e/`, et nulle part ailleurs.
- **`AGENTS.md`, règle 7** : l'image de test aurait un namespace à elle, par exemple
  `cute_disp_e2e` : `cute_display_e2e` fait 16 caractères, au-delà de la limite NVS de
  15. Elle n'en a pas besoin aujourd'hui.
- **`DESIGN.md`** :
  - ligne `firmware` : les images `app`, `hwtest` et `e2e` ;
  - ligne `infrastructure` : `OwnDirectory`, le dossier de l'image sur la carte, et la
    redirection du réseau de l'image de test ;
  - ligne `app` : reçoit le dossier de son image.
- **`docs/testing/README.md`** :
  - corriger la ligne « End-to-end, device » : image dédiée, dossier à elle, l'horloge
    n'est pas remise ;
  - `just e2e-device` n'y entre qu'à l'étape 6, quand la commande existe.

### 2. Le dossier de l'image sur la carte, configurable pour toutes les images

Aujourd'hui `cute-display/` est écrit en dur à plusieurs endroits :
`infrastructure/src/{settings_file,internet,general_file,card_files}.rs`,
`maintenance/src/path.rs` (`DEVICE_DIR`) et `app/tests/image/bench/setup.rs`.

- **`OwnDirectory<S: FileStorage>`** (`src/engine/infrastructure/src/own_directory.rs`) :
  - implémente `FileStorage` sur la carte, toutes les opérations relatives à un dossier
    donné à la construction (`OwnDirectory::new(card, "cute-display")`) ;
  - refuse un chemin qui remonte ou qui est absolu : rien ne sort du dossier.
- **Les consommateurs le reçoivent et parlent en chemins relatifs** :
  - `SettingsFile` : `settings.conf` ;
  - `OnDemandInternet` : `wifi.conf` ;
  - `GeneralFile` : `general.conf` ;
  - `CardFiles` : `apps/<id>`.
  - Les constantes `cute-display/…` disparaissent.
- **`app::run`** reçoit le dossier de l'image (champ de `Devices` ou paramètre) et
  construit l'`OwnDirectory` une seule fois.
- **Chaque composition root donne le sien** :
  - `firmware` image `app` et `simulator` : `cute-display` ;
  - `firmware` image `e2e` : `cute-display-e2e`.
  - Le nom est déclaré dans chaque composition root, sans `cfg` ni mode.
- **Console** (`maintenance`) : elle lit toujours toute la carte (sauvegardes, `cp`
  depuis les `sounds/` d'Habity). Elle n'écrit que dans le dossier de son image, reçu
  par `MaintenanceConsole::new`. `SdPath::is_writable` devient `is_inside(dir)`, et le
  message d'erreur nomme le dossier.
- **Messages des apps** : ceux qui citent un fichier (« put cute-display/wifi.conf »)
  restent tels quels. Ils s'adressent à la personne qui possède l'horloge, donc à l'image
  de prod.
- **Tests sur l'hôte** :
  - `OwnDirectory` passe les `check_*` du contrat `FileStorage` de `hal_testing`, plus un
    test pour un chemin qui tente de sortir ;
  - les tests existants des consommateurs passent sur un `OwnDirectory` ;
  - le banc d'intégration (`app/tests/image/bench/setup.rs`) utilise des chemins
    relatifs au dossier.

### 2 bis. Redirection du réseau de l'image `e2e` (`src/engine/infrastructure/src/testing/to_local_stubs.rs`)

- **`RedirectedHttp<H, S>`** : `https://<hôte>/<chemin>?<q>` devient
  `https://<address>:<https_port>/<hôte>/<chemin>?<q>`. Ce qui n'est pas en `https://`
  est refusé.
- **`RedirectedUdp<U, S>`** : NTP (port 123) part vers `<address>:<ntp_port>`, tout autre
  port est refusé. Un port non privilégié côté PC évite d'avoir besoin de root.
- **Conf** : `e2e.conf` dans le dossier de l'image, avec `address`, `https_port` et
  `ntp_port`, relue à chaque usage comme les autres conf. Sans conf complète, rien ne
  sort et la faute dit quoi poser.
- **Tests** : unitaires sur les stubs HTTP et UDP de `hal_testing`.

### 3. Driver HTTP : choisir à qui faire confiance

- `drivers::esp_http::EspHttpsClient` ne change pas pour l'image `app` : le bundle de CA
  d'ESP-IDF. Sa requête devient `fetch_with(&Configuration, …)`, partagée.
- `drivers::testing::esp_https_one_authority::EspHttpsClientTrustingOneAuthority::new(ca_pem)` :
  `server_certificate` = la CA de test, sans bundle. Dans `testing`, puisque seule
  l'image `e2e` s'en sert (règle 3).
- Rien de testable sur l'hôte : tout est configuration d'ESP-IDF. Vérifié par le build
  ESP32 (`just lint`), et sur l'appareil à l'étape 4.

### 4. Image `e2e` (`src/firmware`)

- La CA : `tools/e2e/src/cute_display_e2e/authority.py` la fait une fois
  (`python -m cute_display_e2e.authority`, `--replace` pour en refaire une) dans
  `tools/e2e/device/ca/` (`ca.pem`, `ca.key`, commités). Il signe aussi les certificats
  du serveur de stubs (`issue_server_pem`), pour l'étape 5.
- `firmware::app_image` : ce que `app` et `e2e` partagent, le `Hardware` générique sur
  `Http` et `Udp`, la console, le logger. `Board` ne porte plus le client HTTPS : il ne
  vient pas du plan des broches.
- `src/bin/e2e.rs` : le dossier `cute-display-e2e`,
  `RedirectedHttp<EspHttpsClientTrustingOneAuthority>` avec `ca.pem` inclus à la
  compilation, `RedirectedUdp<StdUdpClient>`. Sans carte, les redirections le disent et
  rien ne sort (elles prennent un `Result` de carte, comme `Devices::sd_card`).
- La bannière : « Cute Display test image <version> ».
- Sur l'appareil, à vérifier : `just fw-build e2e && just fw-flash e2e`, la bannière,
  `just sd-ls cute-display-e2e`, la météo qui dit « no stubs: put e2e.conf », puis
  `just fw-flash app`.

### 5. Serveurs de stubs (`tools/e2e/src/cute_display_e2e/stub_{web,time}_server.py`)

- `StubWebServer(address, port=8443)` et `StubTimeServer(port=8123)` :
  - HTTPS (`http.server` + `ssl`) : sert `tools/e2e/web/*.http` (URL en première ligne,
    comme le `--web` du simulateur), l'URL d'origine relue dans le `/<hôte>/…` de la
    requête ; 404 pour ce qui n'est pas enregistré ; garde les URL demandées (`asked`).
  - Certificat émis au démarrage pour `address` par `issue_server_pem`.
  - NTP en UDP : l'heure de `FakeRunningTime`, posée par un scénario, qui court ensuite.
  - `e2e_conf(address, https_port, ntp_port)` : le fichier à poser sur la carte.
- `local_address()` : l'adresse du PC sur le réseau local.
- `python -m cute_display_e2e.stub_web_server` : les sert à la main et affiche l'`e2e.conf`.
- Tests dans `tools/e2e/tests/test_stub_{web,time}_server.py` : HTTPS vérifié contre une CA, 404, NTP.
- Pare-feu : aucun actif sur ce PC ; sinon ouvrir 8443/tcp et 8123/udp.
- Vérifié sur l'appareil : le handshake TLS passe avec la CA de test et un hôte en IP
  (la météo de Notre-Dame affichée depuis l'enregistrement), et le stub NTP remet à
  l'heure un RTC décalé d'une heure.

### 6. Harnais pytest sur cible

- `cute_display_link.usb.restart(link)` : RTS tient la puce en reset 0,1 s, DTR bas ;
  le port reste ouvert, la console répond ~2,5 s après.
- `cute_display_e2e.device.Device(link)` : la console (et le log, par `KeepingLog`),
  `restart`, `empty_own_directory_but_the_clocks_wifi` (vide `cute-display-e2e/` et y copie le `wifi.conf` de
  l'horloge, sur la carte), `put`. Sur une autre image, la console refuse d'écrire :
  `NotTheTestImage` dit de flasher l'image de test, sans rien avoir changé. Testé sur
  l'hôte contre le `FakeCard` de `tools/link`, rendu fidèle à la console (dossier en
  écriture configurable, `rm` refusé ailleurs, dossier absent).
- Le log est celui de chaque scénario : `DeviceClock` repart d'un `Log` neuf.
- `scenarios/conftest.py` : `--device`, `--stubs-address`, le marqueur `device` (les
  autres sautés). `DeviceClock` a l'interface de `SimulatedClock` : au premier `start`,
  le dossier vidé ; à chaque `start`, les fichiers posés (carte `standard`, `e2e.conf`,
  ceux du scénario), l'heure des stubs et le RTC à `at`, l'appareil relancé. Les stubs
  sont lancés une fois pour la session.
- Les chemins des scénarios sont relatifs au dossier de l'image. Lire la carte et le RTC
  passe par l'horloge (`clock.read_file`, `clock.rtc_time`), qui connaît son dossier ;
  `Display` ne garde que ce qu'une personne voit et touche.
- `just e2e-device`.
- La console perdait la réponse à la première requête après un temps sans lecteur :
  `ConsoleOutput::heard_from_the_computer` la fait de nouveau attendre de la place.
  Vérifié : 10 commandes sur 10 du premier coup.
- Vérifié sur l'appareil : `test_start` et `test_weather` passent (20 s).

### 7. Premiers scénarios marqués `device`

- Ceux qui ne demandent pas qu'une minute passe : démarrage, réglages, un fetch météo
  (TLS), le radar.
- On vérifie chacun sur l'appareil avant de le marquer.

### 8. Docs

- **`tools/e2e/README.md`**, section « Sur l'appareil » :
  - prérequis : appareil branché, même réseau que le PC, ports `https_port` et
    `ntp_port` ouverts dans le pare-feu ;
  - `just fw-flash e2e`, `just e2e-device` ;
  - remettre la prod avec `just fw-flash app`.
- **Pas d'entrée `CHANGELOG.md` ni `UPDATING.md`** : rien de visible pour les gens qui
  possèdent l'horloge.

## Fichiers critiques

- `AGENTS.md`, `DESIGN.md`, `docs/testing/README.md`, `tools/e2e/README.md`
- `src/engine/infrastructure/src/` : `own_directory.rs` (nouveau), la redirection
  réseau (`testing/to_local_stubs.rs`, nouveau), et `settings_file.rs`, `internet.rs`,
  `general_file.rs`, `card_files.rs`
- `src/app/src/lib.rs`, `src/app/src/engine_services.rs`, `src/simulator/src/main.rs`,
  `src/maintenance/src/{path,console}.rs`
- `src/engine/drivers/src/esp_http.rs`
- `src/firmware/Cargo.toml`, `src/firmware/src/bin/{app,e2e}.rs`,
  `src/firmware/src/lib.rs`
- `tools/e2e/device/ca/`, `tools/e2e/src/cute_display_e2e/{device,stubs}.py`,
  `tools/e2e/scenarios/conftest.py`, `tools/e2e/pyproject.toml`, `justfile`

## Vérification

1. `just test` et `just lint` (hôte + ESP32) passent sans warning.
2. `just e2e` sur le simulateur passe toujours : le harnais refactoré ne casse rien.
3. Sur l'appareil, `just fw-build e2e && just fw-flash e2e`, puis :
   - `just sd-ls cute-display-e2e` montre ce que les scénarios ont posé, et
     `cute-display/` de prod est intact. On le vérifie avec un `just backup-sd` avant et
     après, une fois l'image `app` remise.
   - **Point de risque TLS à valider tôt** : un scénario météo prouve que le handshake
     passe avec la CA de test et un hôte en IP. S'il échoue, on regarde la vérification
     du SAN IP par la version de mbedTLS d'ESP-IDF.
   - `just e2e-device` : les scénarios marqués passent.
4. `just fw-flash app` : Habity et l'app de prod redémarrent normalement.
5. Revue par l'agent `rules-reviewer`.
