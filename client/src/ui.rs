use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui;
use tokio::runtime::Runtime;

use crate::{
    application::{
        authorization_client::AuthorizationClientService,
        ports::{
            RegisterUserResult, RoomRepository, ServerProfileRepository, SessionRepository,
        },
        rooms_client::RoomsClientService,
    },
    domain::{
        authorization::{
            AccountKind, AuthenticationDataInfo, LoginType, LoginUserInfo, RegisterUserInfo,
            ServerCredentials, WhoAmIView,
        },
        rooms::{
            CreateRoomInfo, GetRoomMessagesQuery, JoinRoomInfo, LeaveRoomInfo, RoomListItem,
            RoomMessageDirection, RoomStateEventView, RoomTimelineEventView,
        },
    },
    infrastructure::{
        http::{hyper_authorization_api::HyperAuthorizationApi, hyper_rooms_api::HyperRoomsApi},
        persistence::sqlite_session_repository::SqliteSessionRepository,
    },
};

const PREFERENCE_SELECTED_SERVER_URL: &str = "selected_server_url";
const MESSAGE_PAGE_SIZE: usize = 30;
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(11, 16, 24);
const PANEL: egui::Color32 = egui::Color32::from_rgb(18, 25, 36);
const PANEL_SOFT: egui::Color32 = egui::Color32::from_rgb(25, 34, 48);
const PANEL_HOVER: egui::Color32 = egui::Color32::from_rgb(35, 47, 66);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(89, 214, 190);
const ACCENT_STRONG: egui::Color32 = egui::Color32::from_rgb(247, 198, 103);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(151, 164, 184);
const DANGER: egui::Color32 = egui::Color32::from_rgb(255, 103, 124);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Welcome,
    ChooseFlow,
    RegisterForm,
    Workspace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum UsernameState {
    Unknown,
    Available,
    Unavailable(String),
}

#[derive(Clone, Debug, Default)]
enum LoadState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Error(String),
}

pub struct ClientDesktopApplication {
    runtime: Runtime,
    session_repository: Arc<dyn SessionRepository>,
    server_profile_repository: Arc<dyn ServerProfileRepository>,
    room_repository: Arc<dyn RoomRepository>,
    screen: Screen,
    servers: Vec<String>,
    selected_server_index: usize,
    accounts_for_server: Vec<ServerCredentials>,
    selected_account_index: usize,
    show_add_server_form: bool,
    new_server_url_input: String,
    server_url_input: String,
    username_input: String,
    password_input: String,
    available_auth_flows: Vec<String>,
    selected_auth_flow_index: usize,
    username_state: UsernameState,
    status_message: String,
    whoami: Option<WhoAmIView>,
    rooms: Vec<RoomListItem>,
    selected_room_index: Option<usize>,
    show_create_room_form: bool,
    join_room_id_input: String,
    join_reason_input: String,
    new_room_name: String,
    new_room_topic: String,
    new_room_visibility_index: usize,
    new_room_preset_index: usize,
    new_room_is_direct: bool,
    room_state_events: Vec<RoomStateEventView>,
    room_messages: Vec<RoomTimelineEventView>,
    room_state_status: LoadState,
    room_messages_status: LoadState,
    next_messages_from_token: Option<String>,
    composer_input: String,
    next_transaction_counter: u64,
}

impl ClientDesktopApplication {
    pub fn new(repository: Arc<SqliteSessionRepository>) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|error| error.to_string())?;
        let session_repository: Arc<dyn SessionRepository> = repository.clone();
        let server_profile_repository: Arc<dyn ServerProfileRepository> = repository.clone();
        let room_repository: Arc<dyn RoomRepository> = repository.clone();

        let mut application = Self {
            runtime,
            session_repository,
            server_profile_repository,
            room_repository,
            screen: Screen::Welcome,
            servers: Vec::new(),
            selected_server_index: 0,
            accounts_for_server: Vec::new(),
            selected_account_index: 0,
            show_add_server_form: false,
            new_server_url_input: String::new(),
            server_url_input: "http://127.0.0.1:8080".to_owned(),
            username_input: String::new(),
            password_input: String::new(),
            available_auth_flows: Vec::new(),
            selected_auth_flow_index: 0,
            username_state: UsernameState::Unknown,
            status_message: "Select server and account, then login".to_owned(),
            whoami: None,
            rooms: Vec::new(),
            selected_room_index: None,
            show_create_room_form: false,
            join_room_id_input: String::new(),
            join_reason_input: String::new(),
            new_room_name: String::new(),
            new_room_topic: String::new(),
            new_room_visibility_index: 0,
            new_room_preset_index: 0,
            new_room_is_direct: false,
            room_state_events: Vec::new(),
            room_messages: Vec::new(),
            room_state_status: LoadState::Idle,
            room_messages_status: LoadState::Idle,
            next_messages_from_token: None,
            composer_input: String::new(),
            next_transaction_counter: 0,
        };

        application.reload_servers();
        Ok(application)
    }

    fn normalized_server_url(&self) -> String {
        normalize_server_url_input(&self.server_url_input).unwrap_or_default()
    }

    fn selected_server(&self) -> Option<&str> {
        self.servers
            .get(self.selected_server_index)
            .map(std::string::String::as_str)
    }

    fn selected_account(&self) -> Option<&ServerCredentials> {
        self.accounts_for_server.get(self.selected_account_index)
    }

    fn selected_room(&self) -> Option<&RoomListItem> {
        self.selected_room_index.and_then(|index| self.rooms.get(index))
    }

    fn build_authorization_service(&self) -> AuthorizationClientService {
        let server_url = self
            .selected_server()
            .map(str::to_owned)
            .unwrap_or_else(|| self.normalized_server_url());
        let api = Arc::new(HyperAuthorizationApi::new(server_url.clone()));
        AuthorizationClientService::new(server_url, api, Arc::clone(&self.session_repository))
    }

    fn build_rooms_service(&self) -> RoomsClientService {
        let server_url = self
            .selected_server()
            .map(str::to_owned)
            .unwrap_or_else(|| self.normalized_server_url());
        let rooms_api = Arc::new(HyperRoomsApi::new(server_url.clone()));
        RoomsClientService::new(
            server_url,
            rooms_api,
            Arc::clone(&self.session_repository),
            Arc::clone(&self.room_repository),
        )
    }

    fn preference_selected_username_key(server_url: &str) -> String {
        format!("selected_account_username:{server_url}")
    }

    fn persist_selected_server(&mut self) {
        if let Some(server_url) = self.selected_server().map(str::to_owned) {
            let _ = self.runtime.block_on(
                self.server_profile_repository
                    .set_preference(PREFERENCE_SELECTED_SERVER_URL, &server_url),
            );
        }
    }

    fn persist_selected_account(&mut self) {
        let Some(server_url) = self.selected_server().map(str::to_owned) else {
            return;
        };
        let Some(username) = self.selected_account().map(|account| account.username.clone()) else {
            return;
        };

        let key = Self::preference_selected_username_key(&server_url);
        let _ = self
            .runtime
            .block_on(self.server_profile_repository.set_preference(&key, &username));
    }

    fn reload_servers(&mut self) {
        let selected_server_preference = self.runtime.block_on(
            self.server_profile_repository
                .get_preference(PREFERENCE_SELECTED_SERVER_URL),
        );

        match self
            .runtime
            .block_on(self.server_profile_repository.list_server_credentials())
        {
            Ok(credentials) => {
                let mut servers: Vec<String> = credentials.into_iter().map(|x| x.server_url).collect();
                servers.sort();
                servers.dedup();
                self.servers = servers;

                if let Ok(Some(preferred_server)) = selected_server_preference {
                    if let Some(index) = self
                        .servers
                        .iter()
                        .position(|server| server == &preferred_server)
                    {
                        self.selected_server_index = index;
                    } else if self.selected_server_index >= self.servers.len() {
                        self.selected_server_index = 0;
                    }
                } else if self.selected_server_index >= self.servers.len() {
                    self.selected_server_index = 0;
                }

                if let Some(server) = self.selected_server().map(str::to_owned) {
                    self.server_url_input = server;
                }
                self.reload_accounts_for_selected_server();
            }
            Err(error) => {
                self.status_message = format!("Failed to load servers: {error}");
            }
        }
    }

    fn reload_accounts_for_selected_server(&mut self) {
        let Some(server) = self.selected_server().map(str::to_owned) else {
            self.accounts_for_server.clear();
            self.selected_account_index = 0;
            return;
        };

        let key = Self::preference_selected_username_key(&server);
        let selected_account_preference = self
            .runtime
            .block_on(self.server_profile_repository.get_preference(&key));

        match self
            .runtime
            .block_on(self.server_profile_repository.list_server_accounts(&server))
        {
            Ok(accounts) => {
                self.accounts_for_server = accounts;

                if let Ok(Some(preferred_username)) = selected_account_preference {
                    if let Some(index) = self
                        .accounts_for_server
                        .iter()
                        .position(|account| account.username == preferred_username)
                    {
                        self.selected_account_index = index;
                    } else if self.selected_account_index >= self.accounts_for_server.len() {
                        self.selected_account_index = 0;
                    }
                } else if self.selected_account_index >= self.accounts_for_server.len() {
                    self.selected_account_index = 0;
                }

                if let Some(account) = self.accounts_for_server.get(self.selected_account_index) {
                    self.username_input = account.username.clone();
                    self.password_input = account.password.clone();
                }
            }
            Err(error) => {
                self.status_message = format!("Failed to load accounts: {error}");
            }
        }
    }

    fn add_server(&mut self) {
        let server_url = match normalize_server_url_input(&self.new_server_url_input) {
            Ok(url) => url,
            Err(message) => {
                self.status_message = message;
                return;
            }
        };
        if server_url.is_empty() {
            self.status_message = "Server URL required".to_owned();
            return;
        }

        if !self.servers.iter().any(|known_server| known_server == &server_url) {
            self.servers.push(server_url.clone());
            self.servers.sort();
        }
        if let Some(index) = self.servers.iter().position(|known_server| known_server == &server_url) {
            self.selected_server_index = index;
            self.server_url_input = server_url;
            self.persist_selected_server();
            self.reload_accounts_for_selected_server();
        }

        self.new_server_url_input.clear();
        self.show_add_server_form = false;
        self.status_message = "Server added".to_owned();
    }

    fn save_selected_account_credentials(&mut self) {
        let Some(server_url) = self.selected_server().map(str::to_owned) else {
            self.status_message = "Select server first".to_owned();
            return;
        };

        if self.username_input.trim().is_empty() {
            self.status_message = "Username required".to_owned();
            return;
        }

        let credentials = ServerCredentials {
            server_url: server_url.clone(),
            username: self.username_input.trim().to_owned(),
            password: self.password_input.clone(),
        };
        match self.runtime.block_on(
            self.server_profile_repository
                .upsert_server_credentials(&credentials),
        ) {
            Ok(_) => {
                self.status_message = "Account saved".to_owned();
                self.reload_accounts_for_selected_server();
                if let Some(index) = self
                    .accounts_for_server
                    .iter()
                    .position(|account| account.username == credentials.username)
                {
                    self.selected_account_index = index;
                }
                self.persist_selected_account();
            }
            Err(error) => self.status_message = format!("Failed to save account: {error}"),
        }
    }

    fn run_login_selected_account(&mut self) {
        let Some(account) = self.accounts_for_server.get(self.selected_account_index).cloned() else {
            self.status_message = "Select account first".to_owned();
            return;
        };

        self.username_input = account.username;
        self.password_input = account.password;

        let service = self.build_authorization_service();
        let request = LoginUserInfo {
            login_type: LoginType::Password,
            identifier: None,
            user: Some(self.username_input.trim().to_owned()),
            password: self.password_input.clone(),
            device_identifier: None,
            refresh_token: true,
            token: None,
            medium: None,
            address: None,
            initial_device_display_name: Some("Gridstack Desktop".to_owned()),
        };

        match self.runtime.block_on(service.login_user(&request)) {
            Ok(_) => {
                self.persist_selected_server();
                self.persist_selected_account();
                self.load_whoami();
            }
            Err(error) => self.status_message = format!("Login failed: {error}"),
        }
    }

    fn load_whoami(&mut self) {
        let service = self.build_authorization_service();
        match self.runtime.block_on(service.who_am_i_from_saved_session()) {
            Ok(view) => {
                self.whoami = Some(view);
                self.screen = Screen::Workspace;
                self.status_message = "Login successful".to_owned();
                self.load_rooms();
            }
            Err(error) => {
                self.status_message = format!("Failed to load whoami: {error}");
            }
        }
    }

    fn load_rooms(&mut self) {
        let rooms_service = self.build_rooms_service();
        match self.runtime.block_on(rooms_service.get_joined_rooms()) {
            Ok(joined_rooms) => {
                for room_id in joined_rooms.joined_rooms {
                    let _ = self
                        .runtime
                        .block_on(rooms_service.upsert_cached_room(&room_id, None, None));
                }
            }
            Err(error) => {
                self.status_message = format!(
                    "Failed to refresh joined rooms from server. Showing cached rooms: {error}"
                );
            }
        }

        match self.runtime.block_on(rooms_service.list_cached_rooms()) {
            Ok(items) => {
                let previously_selected_room_id =
                    self.selected_room().map(|room| room.room_id.clone());
                self.rooms = items;

                self.selected_room_index = previously_selected_room_id
                    .and_then(|room_id| self.rooms.iter().position(|room| room.room_id == room_id))
                    .or_else(|| (!self.rooms.is_empty()).then_some(0));

                if self.selected_room_index.is_none() {
                    self.room_state_events.clear();
                    self.room_messages.clear();
                    self.next_messages_from_token = None;
                }
            }
            Err(error) => {
                self.status_message = format!("Failed to load cached rooms: {error}");
            }
        }
    }

    fn start_registration(&mut self) {
        let service = self.build_authorization_service();
        match self.runtime.block_on(service.get_login_flows()) {
            Ok(view) => {
                self.available_auth_flows = view.flows.into_iter().map(|flow| flow.type_field).collect();
                if self.available_auth_flows.is_empty() {
                    self.available_auth_flows.push("m.login.password".to_owned());
                }
                self.selected_auth_flow_index = 0;
                self.username_state = UsernameState::Unknown;
                self.screen = Screen::ChooseFlow;
            }
            Err(error) => self.status_message = format!("Failed to load auth flows: {error}"),
        }
    }

    fn check_username(&mut self) {
        let service = self.build_authorization_service();
        match self
            .runtime
            .block_on(service.check_username_available(self.username_input.trim()))
        {
            Ok(view) => {
                self.username_state = if view.available {
                    UsernameState::Available
                } else {
                    UsernameState::Unavailable("Username is not available".to_owned())
                };
            }
            Err(error) => self.username_state = UsernameState::Unavailable(error.to_string()),
        }
    }

    fn submit_registration(&mut self) {
        if self.username_state != UsernameState::Available {
            self.status_message = "Check username availability before register".to_owned();
            return;
        }

        let service = self.build_authorization_service();
        let first_request = self.build_register_request(None);
        match self
            .runtime
            .block_on(service.register_user(AccountKind::User, &first_request))
        {
            Ok(RegisterUserResult::Registered(_)) => {
                self.save_selected_account_credentials();
                self.persist_selected_server();
                self.persist_selected_account();
                self.load_whoami();
            }
            Ok(RegisterUserResult::AuthenticationRequired(uiaa)) => {
                let flow = self
                    .available_auth_flows
                    .get(self.selected_auth_flow_index)
                    .cloned()
                    .unwrap_or_else(|| "m.login.password".to_owned());
                let second_request = self.build_register_request(Some(AuthenticationDataInfo {
                    session: Some(uiaa.session),
                    authentication_type: Some(flow),
                    data: BTreeMap::new(),
                }));

                match self
                    .runtime
                    .block_on(service.register_user(AccountKind::User, &second_request))
                {
                    Ok(RegisterUserResult::Registered(_)) => {
                        self.save_selected_account_credentials();
                        self.persist_selected_server();
                        self.persist_selected_account();
                        self.load_whoami();
                    }
                    Ok(RegisterUserResult::AuthenticationRequired(_)) => {
                        self.status_message = "Additional auth steps required".to_owned();
                    }
                    Err(error) => self.status_message = format!("Registration failed: {error}"),
                }
            }
            Err(error) => self.status_message = format!("Registration failed: {error}"),
        }
    }

    fn build_register_request(
        &self,
        authentication: Option<AuthenticationDataInfo>,
    ) -> RegisterUserInfo {
        RegisterUserInfo {
            authentication,
            device_identifier: None,
            inhibit_login: false,
            initial_device_display_name: Some("Gridstack Desktop".to_owned()),
            password: Some(self.password_input.clone()),
            refresh_token: true,
            username: Some(self.username_input.trim().to_owned()),
        }
    }

    fn run_logout(&mut self) {
        let service = self.build_authorization_service();
        match self.runtime.block_on(service.logout_from_saved_session()) {
            Ok(_) => {
                self.screen = Screen::Welcome;
                self.status_message = "Logged out".to_owned();
                self.rooms.clear();
                self.selected_room_index = None;
                self.room_state_events.clear();
                self.room_messages.clear();
                self.room_state_status = LoadState::Idle;
                self.room_messages_status = LoadState::Idle;
                self.next_messages_from_token = None;
                self.whoami = None;
                self.composer_input.clear();
            }
            Err(error) => self.status_message = format!("Logout failed: {error}"),
        }
    }

    fn create_room(&mut self) {
        let rooms_service = self.build_rooms_service();
        let visibility = Self::visibility_options()
            .get(self.new_room_visibility_index)
            .copied()
            .unwrap_or("private")
            .to_owned();
        let preset = Self::available_presets_for_visibility(visibility.as_str())
            .get(self.new_room_preset_index)
            .copied()
            .unwrap_or("private_chat")
            .to_owned();
        let request = CreateRoomInfo {
            name: if self.new_room_name.trim().is_empty() {
                None
            } else {
                Some(self.new_room_name.trim().to_owned())
            },
            topic: if self.new_room_topic.trim().is_empty() {
                None
            } else {
                Some(self.new_room_topic.trim().to_owned())
            },
            visibility: Some(visibility),
            preset: Some(preset),
            room_alias_name: None,
            room_version: None,
            is_direct: Some(self.new_room_is_direct),
            creation_content: None,
            invite: None,
            invite_3pid: None,
            power_level_content_override: None,
        };

        match self.runtime.block_on(rooms_service.create_room(&request)) {
            Ok(view) => {
                self.status_message = format!("Room created: {}", view.room_id);
                self.new_room_name.clear();
                self.new_room_topic.clear();
                self.show_create_room_form = false;
                self.load_rooms();
                self.select_room_by_id(&view.room_id);
            }
            Err(error) => {
                self.status_message = format!("Create room failed: {error}");
            }
        }
    }

    fn join_room_by_id(&mut self) {
        let room_id = self.join_room_id_input.trim().to_owned();
        if room_id.is_empty() {
            self.status_message = "Room ID is required for join".to_owned();
            return;
        }

        let request = JoinRoomInfo {
            reason: (!self.join_reason_input.trim().is_empty())
                .then(|| self.join_reason_input.trim().to_owned()),
        };

        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.join_room_by_id(&room_id, &request))
        {
            Ok(view) => {
                self.status_message = format!("Joined room: {}", view.room_id);
                self.join_room_id_input.clear();
                self.join_reason_input.clear();
                self.load_rooms();
                self.select_room_by_id(&view.room_id);
            }
            Err(error) => {
                self.status_message = format!("Join room failed: {error}");
            }
        }
    }

    fn leave_selected_room(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room first".to_owned();
            return;
        };

        let request = LeaveRoomInfo {
            reason: (!self.join_reason_input.trim().is_empty())
                .then(|| self.join_reason_input.trim().to_owned()),
        };
        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.leave_room_by_id(&room_id, &request))
        {
            Ok(_) => {
                self.status_message = format!("Left room: {room_id}");
                self.load_rooms();
                self.room_state_events.clear();
                self.room_messages.clear();
                self.room_state_status = LoadState::Idle;
                self.room_messages_status = LoadState::Idle;
                self.next_messages_from_token = None;
            }
            Err(error) => self.status_message = format!("Leave room failed: {error}"),
        }
    }

    fn select_room_by_id(&mut self, room_id: &str) {
        if self.set_selected_room_index_by_id(room_id) {
            self.load_selected_room_details();
        }
    }

    fn set_selected_room_index_by_id(&mut self, room_id: &str) -> bool {
        if let Some(index) = self.rooms.iter().position(|room| room.room_id == room_id) {
            self.selected_room_index = Some(index);
            return true;
        }
        false
    }

    fn load_selected_room_details(&mut self) {
        self.load_selected_room_state();
        self.load_selected_room_messages(None, true);
    }

    fn load_selected_room_state(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.room_state_status = LoadState::Idle;
            return;
        };

        self.room_state_status = LoadState::Loading;
        let rooms_service = self.build_rooms_service();
        match self.runtime.block_on(rooms_service.get_room_state(&room_id)) {
            Ok(events) => {
                self.room_state_events = events;
                self.room_state_status = LoadState::Loaded;

                let (name, topic) = extract_room_name_and_topic(&self.room_state_events);
                if name.is_some() || topic.is_some() {
                    let _ = self.runtime.block_on(rooms_service.upsert_cached_room(
                        &room_id,
                        name,
                        topic,
                    ));
                    self.load_rooms();
                    let _ = self.set_selected_room_index_by_id(&room_id);
                }
            }
            Err(error) => {
                self.room_state_events.clear();
                self.room_state_status = LoadState::Error(error.to_string());
            }
        }
    }

    fn load_selected_room_messages(&mut self, from_token: Option<String>, replace: bool) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.room_messages_status = LoadState::Idle;
            return;
        };

        self.room_messages_status = LoadState::Loading;
        let query = GetRoomMessagesQuery {
            from_token,
            to_token: None,
            direction: RoomMessageDirection::Backward,
            limit: MESSAGE_PAGE_SIZE,
            filter: None,
        };
        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.get_room_messages(&room_id, &query))
        {
            Ok(page) => {
                self.next_messages_from_token = page.end;
                if replace {
                    self.room_messages = page.chunk;
                } else {
                    self.room_messages.extend(page.chunk);
                }
                self.room_messages_status = LoadState::Loaded;
            }
            Err(error) => {
                self.room_messages_status = LoadState::Error(error.to_string());
            }
        }
    }

    fn load_older_messages(&mut self) {
        let Some(token) = self.next_messages_from_token.clone() else {
            self.status_message = "No older messages token available for this room".to_owned();
            return;
        };
        self.load_selected_room_messages(Some(token), false);
    }

    fn send_message(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before sending a message".to_owned();
            return;
        };

        let message_body = self.composer_input.trim().to_owned();
        if message_body.is_empty() {
            self.status_message = "Message text is required".to_owned();
            return;
        }

        let transaction_id = self.build_transaction_id();
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": message_body,
        });
        let rooms_service = self.build_rooms_service();

        match self.runtime.block_on(rooms_service.send_room_message_event(
            &room_id,
            "m.room.message",
            &transaction_id,
            &content,
        )) {
            Ok(response) => {
                self.status_message = format!("Message sent ({})", response.event_id);
                self.composer_input.clear();
                self.load_selected_room_messages(None, true);
            }
            Err(error) => {
                self.status_message = format!("Send message failed: {error}");
            }
        }
    }

    fn build_transaction_id(&mut self) -> String {
        self.next_transaction_counter = self.next_transaction_counter.saturating_add(1);
        let timestamp_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        format!(
            "client-{}-{}",
            timestamp_millis, self.next_transaction_counter
        )
    }

    fn render_welcome(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(36.0);
            ui.label(
                egui::RichText::new("GRIDSTACK")
                    .size(12.0)
                    .color(ACCENT)
                    .strong(),
            );
            ui.heading(egui::RichText::new("Secure rooms, local first").size(34.0));
            ui.label(
                egui::RichText::new("Choose a homeserver and continue with a saved account.")
                    .color(TEXT_MUTED),
            );
            ui.add_space(22.0);
        });

        egui::Frame::default()
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, PANEL_HOVER))
            .corner_radius(egui::CornerRadius::same(18))
            .inner_margin(egui::Margin::symmetric(22, 20))
            .show(ui, |ui| {
                ui.set_max_width(620.0);
                ui.label(
                    egui::RichText::new("Server")
                        .size(13.0)
                        .color(TEXT_MUTED)
                        .strong(),
                );
                ui.horizontal(|ui| {
                    if !self.servers.is_empty() {
                        let mut changed_to_index: Option<usize> = None;
                        egui::ComboBox::from_id_salt("welcome_server")
                            .width(440.0)
                            .selected_text(
                                self.servers
                                    .get(self.selected_server_index)
                                    .cloned()
                                    .unwrap_or_else(|| "Choose server".to_owned()),
                            )
                            .show_ui(ui, |ui| {
                                for (index, server) in self.servers.iter().enumerate() {
                                    if ui
                                        .selectable_label(self.selected_server_index == index, server)
                                        .clicked()
                                    {
                                        changed_to_index = Some(index);
                                    }
                                }
                            });
                        if let Some(index) = changed_to_index {
                            self.selected_server_index = index;
                            if let Some(server) = self.servers.get(index) {
                                self.server_url_input = server.clone();
                            }
                            self.persist_selected_server();
                            self.reload_accounts_for_selected_server();
                        }
                    } else {
                        ui.label(egui::RichText::new("No saved servers").color(TEXT_MUTED));
                    }

                    if ui.button("+ Server").clicked() {
                        self.show_add_server_form = true;
                    }
                });

                if self.show_add_server_form {
                    ui.add_space(10.0);
                    egui::Frame::default()
                        .fill(PANEL_SOFT)
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.label("Add server");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_server_url_input)
                                    .hint_text("100.x.y.z:3000 or http://100.x.y.z:3000"),
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    self.add_server();
                                }
                                if ui.button("Cancel").clicked() {
                                    self.show_add_server_form = false;
                                }
                            });
                        });
                }

                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("Account")
                        .size(13.0)
                        .color(TEXT_MUTED)
                        .strong(),
                );
                if !self.accounts_for_server.is_empty() {
                    let mut changed_account_index: Option<usize> = None;
                    egui::ComboBox::from_id_salt("welcome_account")
                        .width(440.0)
                        .selected_text(
                            self.accounts_for_server
                                .get(self.selected_account_index)
                                .map(|account| account.username.clone())
                                .unwrap_or_else(|| "Choose account".to_owned()),
                        )
                        .show_ui(ui, |ui| {
                            for (index, account) in self.accounts_for_server.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        self.selected_account_index == index,
                                        &account.username,
                                    )
                                    .clicked()
                                {
                                    changed_account_index = Some(index);
                                }
                            }
                        });
                    if let Some(index) = changed_account_index {
                        self.selected_account_index = index;
                        if let Some(account) = self.accounts_for_server.get(index) {
                            self.username_input = account.username.clone();
                            self.password_input = account.password.clone();
                        }
                        self.persist_selected_account();
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Login").clicked() {
                            self.run_login_selected_account();
                        }
                        if ui.button("Register new account").clicked() {
                            self.start_registration();
                        }
                    });
                } else {
                    ui.label(egui::RichText::new("No saved accounts for this server.").color(TEXT_MUTED));
                    ui.add_space(8.0);
                    ui.add(egui::TextEdit::singleline(&mut self.username_input).hint_text("Username"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password_input)
                            .password(true)
                            .hint_text("Password"),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Save account").clicked() {
                            self.save_selected_account_credentials();
                        }
                        if ui.button("Register").clicked() {
                            self.start_registration();
                        }
                    });
                }
            });
    }

    fn render_choose_flow(&mut self, ui: &mut egui::Ui) {
        ui.heading("Choose Auth Flow");
        ui.label(format!("Server: {}", self.selected_server().unwrap_or("-")));

        egui::ComboBox::from_label("Available flows")
            .selected_text(
                self.available_auth_flows
                    .get(self.selected_auth_flow_index)
                    .cloned()
                    .unwrap_or_else(|| "m.login.password".to_owned()),
            )
            .show_ui(ui, |ui| {
                for (index, flow) in self.available_auth_flows.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_auth_flow_index, index, flow);
                }
            });

        ui.horizontal(|ui| {
            if ui.button("Continue").clicked() {
                self.screen = Screen::RegisterForm;
            }
            if ui.button("Back").clicked() {
                self.screen = Screen::Welcome;
            }
        });
    }

    fn render_register_form(&mut self, ui: &mut egui::Ui) {
        ui.heading("Register");
        ui.horizontal(|ui| {
            ui.label("Username");
            let response = ui.text_edit_singleline(&mut self.username_input);
            if response.changed() {
                self.username_state = UsernameState::Unknown;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Password");
            ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));
        });

        ui.horizontal(|ui| {
            if ui.button("Check username").clicked() {
                self.check_username();
            }

            match &self.username_state {
                UsernameState::Unknown => {
                    ui.label("Not checked");
                }
                UsernameState::Available => {
                    ui.colored_label(egui::Color32::GREEN, "Available");
                }
                UsernameState::Unavailable(reason) => {
                    ui.colored_label(egui::Color32::RED, reason);
                }
            }
        });

        let can_register = self.username_state == UsernameState::Available;
        if ui
            .add_enabled(can_register, egui::Button::new("Register"))
            .clicked()
        {
            self.submit_registration();
        }

        if ui.button("Back").clicked() {
            self.screen = Screen::ChooseFlow;
        }
    }

    fn render_workspace(&mut self, context: &egui::Context) {
        egui::SidePanel::left("room_sidebar")
            .resizable(true)
            .default_width(330.0)
            .frame(
                egui::Frame::default()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 16)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Gridstack")
                            .size(24.0)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Logout").clicked() {
                            self.run_logout();
                        }
                    });
                });
                if let Some(whoami) = self.whoami.as_ref() {
                    ui.label(egui::RichText::new(&whoami.user_id).color(TEXT_MUTED));
                }
                ui.add_space(16.0);

                egui::Frame::default()
                    .fill(PANEL_SOFT)
                    .corner_radius(egui::CornerRadius::same(16))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Session").color(TEXT_MUTED).strong());
                        let mut changed_server_index: Option<usize> = None;
                        egui::ComboBox::from_id_salt("workspace_server")
                            .width(ui.available_width())
                            .selected_text(
                                self.servers
                                    .get(self.selected_server_index)
                                    .cloned()
                                    .unwrap_or_else(|| "Choose server".to_owned()),
                            )
                            .show_ui(ui, |ui| {
                                for (index, server) in self.servers.iter().enumerate() {
                                    if ui
                                        .selectable_label(self.selected_server_index == index, server)
                                        .clicked()
                                    {
                                        changed_server_index = Some(index);
                                    }
                                }
                            });
                        if let Some(index) = changed_server_index {
                            self.selected_server_index = index;
                            self.persist_selected_server();
                            self.reload_accounts_for_selected_server();
                            self.load_rooms();
                        }

                        let mut changed_account_index: Option<usize> = None;
                        egui::ComboBox::from_id_salt("workspace_account")
                            .width(ui.available_width())
                            .selected_text(
                                self.accounts_for_server
                                    .get(self.selected_account_index)
                                    .map(|account| account.username.clone())
                                    .unwrap_or_else(|| "Choose account".to_owned()),
                            )
                            .show_ui(ui, |ui| {
                                for (index, account) in self.accounts_for_server.iter().enumerate() {
                                    if ui
                                        .selectable_label(
                                            self.selected_account_index == index,
                                            &account.username,
                                        )
                                        .clicked()
                                    {
                                        changed_account_index = Some(index);
                                    }
                                }
                            });
                        if let Some(index) = changed_account_index {
                            self.selected_account_index = index;
                            if let Some(account) = self.accounts_for_server.get(index) {
                                self.username_input = account.username.clone();
                                self.password_input = account.password.clone();
                            }
                            self.persist_selected_account();
                        }

                        if ui.button("Refresh cached rooms").clicked() {
                            self.load_rooms();
                        }
                    });

                ui.add_space(12.0);
                ui.collapsing("Join by room id", |ui| {
                    ui.text_edit_singleline(&mut self.join_room_id_input);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.join_reason_input)
                            .hint_text("Reason, optional"),
                    );
                    if ui.button("Join").clicked() {
                        self.join_room_by_id();
                    }
                });

                if ui
                    .button(if self.show_create_room_form {
                        "Hide create room"
                    } else {
                        "Create room"
                    })
                    .clicked()
                {
                    self.show_create_room_form = !self.show_create_room_form;
                }
                if self.show_create_room_form {
                    egui::Frame::default()
                        .fill(PANEL_SOFT)
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.new_room_name).hint_text("Room name"));
                        ui.add(egui::TextEdit::singleline(&mut self.new_room_topic).hint_text("Topic"));

                        let visibility_options = Self::visibility_options();
                        egui::ComboBox::from_label("Visibility")
                            .selected_text(
                                visibility_options
                                    .get(self.new_room_visibility_index)
                                    .copied()
                                    .unwrap_or("private"),
                            )
                            .show_ui(ui, |ui| {
                                for (index, value) in visibility_options.iter().enumerate() {
                                    if ui
                                        .selectable_value(&mut self.new_room_visibility_index, index, *value)
                                        .clicked()
                                    {
                                        self.enforce_preset_for_visibility();
                                    }
                                }
                            });

                        let active_visibility = visibility_options
                            .get(self.new_room_visibility_index)
                            .copied()
                            .unwrap_or("private");
                        let preset_options =
                            Self::available_presets_for_visibility(active_visibility);
                        if self.new_room_preset_index >= preset_options.len() {
                            self.new_room_preset_index = 0;
                        }
                        egui::ComboBox::from_label("Preset")
                            .selected_text(
                                preset_options
                                    .get(self.new_room_preset_index)
                                    .copied()
                                    .unwrap_or("private_chat"),
                            )
                            .show_ui(ui, |ui| {
                                for (index, value) in preset_options.iter().enumerate() {
                                    if ui
                                        .selectable_value(&mut self.new_room_preset_index, index, *value)
                                        .clicked()
                                    {
                                        self.enforce_visibility_for_preset();
                                    }
                                }
                            });
                        ui.checkbox(&mut self.new_room_is_direct, "Direct room");
                        if ui.button("Create").clicked() {
                            self.create_room();
                        }
                    });
                }

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Rooms")
                            .size(13.0)
                            .color(TEXT_MUTED)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(self.rooms.len().to_string()).color(ACCENT));
                    });
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for index in 0..self.rooms.len() {
                        let room = &self.rooms[index];
                        let room_title = room.name.clone().unwrap_or_else(|| room.room_id.clone());
                        let topic = room.topic.clone().unwrap_or_else(|| room.room_id.clone());
                        let selected = self.selected_room_index == Some(index);
                        let fill = if selected { PANEL_HOVER } else { PANEL };
                        let response = egui::Frame::default()
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                1.0,
                                if selected { ACCENT } else { PANEL_SOFT },
                            ))
                            .corner_radius(egui::CornerRadius::same(14))
                            .inner_margin(egui::Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    avatar(ui, room_title.chars().next().unwrap_or('#'), selected);
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(room_title).strong());
                                        ui.label(
                                            egui::RichText::new(compact_text(&topic, 38))
                                                .size(11.0)
                                                .color(TEXT_MUTED),
                                        );
                                    });
                                });
                            })
                            .response;
                        if response.interact(egui::Sense::click()).clicked() {
                            self.selected_room_index = Some(index);
                            self.load_selected_room_details();
                        }
                        ui.add_space(6.0);
                    }
                });

                if ui.button("Leave selected room").clicked() {
                    self.leave_selected_room();
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(BACKGROUND))
            .show(context, |ui| {
            let selected_room_id = self.selected_room().map(|room| room.room_id.clone());
            let selected_room_name = self
                .selected_room()
                .and_then(|room| room.name.clone())
                .unwrap_or_else(|| selected_room_id.clone().unwrap_or_else(|| "-".to_owned()));

            egui::Frame::default()
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        avatar(ui, selected_room_name.chars().next().unwrap_or('#'), true);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(selected_room_name)
                                    .size(20.0)
                                    .strong(),
                            );
                            if let Some(room_id) = selected_room_id.as_ref() {
                                ui.label(
                                    egui::RichText::new(compact_text(room_id, 84))
                                        .size(11.0)
                                        .color(TEXT_MUTED),
                                );
                            } else {
                                ui.label(egui::RichText::new("Select a room").color(TEXT_MUTED));
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add_enabled(
                                    self.next_messages_from_token.is_some(),
                                    egui::Button::new("Older"),
                                )
                                .clicked()
                            {
                                self.load_older_messages();
                            }
                            if ui.button("Messages").clicked() {
                                self.load_selected_room_messages(None, true);
                            }
                            if ui.button("State").clicked() {
                                self.load_selected_room_state();
                            }
                        });
                    });
                });

            ui.add_space(12.0);
            egui::Frame::default()
                .fill(PANEL_SOFT)
                .corner_radius(egui::CornerRadius::same(16))
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
            ui.collapsing("Room state", |ui| match &self.room_state_status {
                LoadState::Idle => {
                    ui.label(egui::RichText::new("No room selected.").color(TEXT_MUTED));
                }
                LoadState::Loading => {
                    ui.label(egui::RichText::new("Loading room state...").color(TEXT_MUTED));
                }
                LoadState::Error(error) => {
                    ui.colored_label(DANGER, format!("Failed to load state: {error}"));
                }
                LoadState::Loaded => {
                    if self.room_state_events.is_empty() {
                        ui.label(egui::RichText::new("This room has no state events.").color(TEXT_MUTED));
                    } else {
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for event in &self.room_state_events {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} [{}]",
                                        event.event_type, event.state_key
                                    ))
                                    .color(ACCENT_STRONG)
                                    .strong(),
                                );
                                ui.label(egui::RichText::new(&event.sender).color(TEXT_MUTED));
                                ui.monospace(pretty_json(&event.content));
                                ui.separator();
                            }
                        });
                    }
                }
            });
                });

            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("Messages")
                    .size(14.0)
                    .color(TEXT_MUTED)
                    .strong(),
            );
            match &self.room_messages_status {
                LoadState::Idle => {
                    empty_chat(ui, "Select a room from the sidebar to inspect its timeline.");
                }
                LoadState::Loading => {
                    empty_chat(ui, "Loading room messages...");
                }
                LoadState::Error(error) => {
                    ui.colored_label(DANGER, format!("Failed to load messages: {error}"));
                }
                LoadState::Loaded => {
                    if self.room_messages.is_empty() {
                        empty_chat(ui, "No timeline messages returned for this room.");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for event in &self.room_messages {
                                render_timeline_event(ui, event, self.whoami.as_ref());
                                ui.add_space(10.0);
                            }
                        });
                    }
                }
            }

            ui.add_space(10.0);
            egui::Frame::default()
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    let can_send = self.selected_room().is_some() && !self.composer_input.trim().is_empty();
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.composer_input)
                                .hint_text("Write a message"),
                        );
                        if ui
                            .add_enabled(can_send, egui::Button::new("Send"))
                            .clicked()
                        {
                            self.send_message();
                        }
                    });
                    if self.selected_room().is_none() {
                        ui.label(
                            egui::RichText::new("Select a room to enable message sending.")
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "Uses `PUT /rooms/{roomId}/send/m.room.message/{txnId}`.",
                            )
                            .size(11.0)
                            .color(TEXT_MUTED),
                        );
                    }
                });
        });
    }

    fn visibility_options() -> &'static [&'static str] {
        &["private", "public"]
    }

    fn available_presets_for_visibility(visibility: &str) -> &'static [&'static str] {
        match visibility {
            "public" => &["public_chat"],
            _ => &["private_chat", "trusted_private_chat"],
        }
    }

    fn enforce_preset_for_visibility(&mut self) {
        let visibility = Self::visibility_options()
            .get(self.new_room_visibility_index)
            .copied()
            .unwrap_or("private");
        let presets = Self::available_presets_for_visibility(visibility);
        if self.new_room_preset_index >= presets.len() {
            self.new_room_preset_index = 0;
        }
    }

    fn enforce_visibility_for_preset(&mut self) {
        let private_presets = ["private_chat", "trusted_private_chat"];
        let current_visibility = Self::visibility_options()
            .get(self.new_room_visibility_index)
            .copied()
            .unwrap_or("private");
        let preset = Self::available_presets_for_visibility(current_visibility)
            .get(self.new_room_preset_index)
            .copied()
            .unwrap_or("private_chat");
        self.new_room_visibility_index = if private_presets.contains(&preset) { 0 } else { 1 };
    }
}

impl eframe::App for ClientDesktopApplication {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        apply_visual_theme(context);

        match self.screen {
            Screen::Workspace => self.render_workspace(context),
            _ => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::default().fill(BACKGROUND))
                    .show(context, |ui| match self.screen {
                    Screen::Welcome => self.render_welcome(ui),
                    Screen::ChooseFlow => self.render_choose_flow(ui),
                    Screen::RegisterForm => self.render_register_form(ui),
                    Screen::Workspace => {}
                });
            }
        }

        egui::TopBottomPanel::bottom("status_bar").show(context, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&self.status_message).color(TEXT_MUTED));
            });
        });
    }
}

fn apply_visual_theme(context: &egui::Context) {
    let mut style = (*context.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = BACKGROUND;
    style.visuals.widgets.noninteractive.bg_fill = PANEL;
    style.visuals.widgets.inactive.bg_fill = PANEL_SOFT;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    style.visuals.widgets.hovered.bg_fill = PANEL_HOVER;
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.selection.bg_fill = ACCENT;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    context.set_style(style);
}

fn avatar(ui: &mut egui::Ui, initial: char, highlighted: bool) {
    let color = if highlighted { ACCENT } else { PANEL_HOVER };
    let text_color = if highlighted {
        BACKGROUND
    } else {
        egui::Color32::WHITE
    };
    egui::Frame::default()
        .fill(color)
        .corner_radius(egui::CornerRadius::same(40))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(initial.to_uppercase().to_string())
                    .color(text_color)
                    .strong(),
            );
        });
}

fn empty_chat(ui: &mut egui::Ui, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        ui.label(egui::RichText::new("No active timeline").size(22.0).strong());
        ui.label(egui::RichText::new(message).color(TEXT_MUTED));
    });
}

fn render_timeline_event(
    ui: &mut egui::Ui,
    event: &RoomTimelineEventView,
    whoami: Option<&WhoAmIView>,
) {
    let sent_by_me = whoami
        .map(|view| view.user_id == event.sender)
        .unwrap_or(false);
    let fill = if sent_by_me { ACCENT } else { PANEL_SOFT };
    let text_color = if sent_by_me {
        BACKGROUND
    } else {
        egui::Color32::WHITE
    };
    let body = message_body(event);

    ui.with_layout(
        if sent_by_me {
            egui::Layout::right_to_left(egui::Align::Min)
        } else {
            egui::Layout::left_to_right(egui::Align::Min)
        },
        |ui| {
            egui::Frame::default()
                .fill(fill)
                .stroke(egui::Stroke::new(
                    1.0,
                    if sent_by_me { ACCENT } else { PANEL_HOVER },
                ))
                .corner_radius(egui::CornerRadius::same(18))
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_max_width(560.0);
                    ui.label(egui::RichText::new(body).color(text_color));
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(compact_text(&event.sender, 42))
                                .size(10.0)
                                .color(if sent_by_me { BACKGROUND } else { TEXT_MUTED }),
                        );
                        ui.label(
                            egui::RichText::new(&event.event_type)
                                .size(10.0)
                                .color(if sent_by_me { BACKGROUND } else { ACCENT_STRONG }),
                        );
                    });
                });
        },
    );
}

fn extract_room_name_and_topic(events: &[RoomStateEventView]) -> (Option<String>, Option<String>) {
    let mut room_name: Option<String> = None;
    let mut room_topic: Option<String> = None;

    for event in events {
        if event.state_key.is_empty() && event.event_type == "m.room.name" {
            if let Some(name) = event.content.get("name").and_then(serde_json::Value::as_str) {
                room_name = Some(name.to_owned());
            }
        }
        if event.state_key.is_empty() && event.event_type == "m.room.topic" {
            if let Some(topic) = event.content.get("topic").and_then(serde_json::Value::as_str) {
                room_topic = Some(topic.to_owned());
            }
        }
    }

    (room_name, room_topic)
}

fn pretty_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn normalize_server_url_input(input: &str) -> Result<String, String> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Server URL required".to_owned());
    }

    let with_scheme = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    };

    match with_scheme.parse::<http::Uri>() {
        Ok(uri) => {
            if uri.scheme_str().is_none() || uri.authority().is_none() {
                return Err(
                    "Invalid server URL. Use format like http://100.x.y.z:3000".to_owned(),
                );
            }
            Ok(with_scheme)
        }
        Err(_) => Err("Invalid server URL. Use format like http://100.x.y.z:3000".to_owned()),
    }
}

fn message_body(event: &RoomTimelineEventView) -> String {
    event
        .content
        .get("body")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| pretty_json(&event.content))
}

fn compact_text(value: &str, maximum_characters: usize) -> String {
    if value.chars().count() <= maximum_characters {
        return value.to_owned();
    }

    let mut shortened = value
        .chars()
        .take(maximum_characters.saturating_sub(3))
        .collect::<String>();
    shortened.push_str("...");
    shortened
}
