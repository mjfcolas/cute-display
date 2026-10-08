# Tests end-to-end sur cible : image `e2e`, stubs réseau sur le LAN

## Avancement

- [x] 1. Règles et design
- [x] 2. Le dossier de l'image sur la carte, configurable pour toutes les images
- [x] 2 bis. Redirection du réseau de l'image `e2e`
- [ ] 3. Driver HTTP : choisir à qui faire confiance
- [ ] 4. Image `e2e`
- [ ] 5. Serveur de stubs
- [ ] 6. Harnais pytest sur cible
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

### 3. Driver HTTP : choisir à qui faire confiance (`src/engine/drivers/src/esp_http.rs`)

- Deux constructeurs :
  - `EspHttpsClient::trusting_espidf_bundle()` : le comportement actuel ;
  - `EspHttpsClient::trusting_only(ca: &'static CStr)` : `server_certificate`, sans
    bundle.
- L'image `app` est inchangée dans les faits.

### 4. Image `e2e` (`src/firmware`)

- Nouveau `[[bin]] e2e` dans `src/firmware/Cargo.toml`, en `src/bin/e2e.rs`.
- Ce que `app.rs` et `e2e.rs` partagent passe dans la lib `firmware` : le `Hardware`
  générique ou ses morceaux, `spawn_speaker`, `start_maintenance`.
- `e2e.rs` câble :
  - le dossier `cute-display-e2e`, pour l'app et pour la console ;
  - `RedirectedHttp<EspHttpsClient::trusting_only(CA)>` ;
  - `RedirectedUdp<StdUdpClient>`.
- La CA est incluse avec `include_str!` depuis `tools/e2e/device/ca/ca.pem`, plus un NUL
  final.
- La bannière de log et la version disent `e2e`, pour qu'on voie tout de suite quelle
  image tourne.
- Sans `e2e.conf` lisible : on logge une faute et le réseau reste injoignable. Pas de
  panic (règle 4).
- `just fw-build e2e` et `just fw-flash e2e` marchent déjà, le binaire étant en
  paramètre. Il faut vérifier que l'installeur (`cute_display install`) accepte l'image.

### 5. Serveur de stubs (`tools/e2e/src/cute_display_e2e/stubs.py`)

- **HTTPS** (`http.server` + `ssl`) :
  - sert `tools/e2e/web/*.http`, dont la première ligne est l'URL et le reste le corps,
    comme dans `src/simulator/src/recordings.rs` ;
  - retrouve l'URL d'origine en relisant le `/<hôte>/…` de la requête ;
  - répond 404 pour une URL non enregistrée, comme `RecordedWeb`.
- **Certificat serveur** : émis à chaque session pour l'IP, signé par la CA, avec la lib
  `cryptography` (dépendance de `tools/e2e/pyproject.toml`).
- **NTP en UDP** : répond l'heure du scénario plus le temps écoulé. C'est le même paquet
  que `testing::ntp_server::answer`, porté en Python.
- **Tests** dans `tools/e2e/tests/` (lancés par `just test`) :
  - une requête HTTPS qui vérifie le certificat avec la CA ;
  - un 404 ;
  - le décodage d'une réponse NTP.

### 6. Harnais pytest sur cible

- **`tools/e2e/src/cute_display_e2e/device.py`** : `Device`, l'équivalent de
  `Simulator`.
  - `link` : `cute_display_link.usb.open_link()`. Ouvrir le port reset la puce, voir le
    commentaire dans `usb.py`.
  - `log` : `Log`, alimenté par les lignes de log que `console.KeepingLog` met de côté.
  - `crashes()` reconnaît en plus `Guru Meditation`, `stack overflow` et un reboot
    inattendu, c'est-à-dire une deuxième bannière de boot.
- **Dans `scenarios/conftest.py`** :
  - option `--device` ;
  - fixture `clock` qui rend un `SimulatedClock` ou un `DeviceClock` avec la même
    interface : `put_before_start`, `start(at=…)`, `logs`, `check_and_close`.
- **Chemins relatifs au dossier de l'image** : les scénarios passent
  `general.conf`, `apps/timer/timer.conf`, etc., sans `cute-display/`. Chaque clock les
  préfixe de son dossier : `cute-display` pour le simulateur, `cute-display-e2e` pour
  l'appareil.
- **`DeviceClock.start(at)`** :
  1. vide `cute-display-e2e/` par la console ;
  2. y pose `cards/standard`, `wifi.conf`, `e2e.conf` et les fichiers du scénario ;
  3. règle le RTC à `at` (`clock set`) et le stub NTP sur `at` ;
  4. reset la puce et attend la console.
- **Une seule session de stubs** par run pytest (fixture de session).
- **IP du PC** : détectée par la route vers le LAN, ou donnée par `--stubs-address`.
  Elle est écrite dans `e2e.conf` avant les tests.
- **`wifi.conf` du réseau de test** : `--wifi PATH`, par défaut
  `backup/sd/cute-display/wifi.conf` (créé par `just backup-sd`).
- **Sélection des scénarios** :
  - marqueur `@pytest.mark.device` sur les scénarios assez rapides pour la cible ;
  - avec `--device`, les autres sont ignorés ;
  - à ajuster selon `SPEED` : sur cible, la vitesse est 1.
- **Délais** : `Display` et `waiting.until` prennent des délais propres à la cible
  (rafraîchissement réel, montée du Wi-Fi).
- **`justfile`** : `e2e-device *args`, qui lance pytest avec `--device`.

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
