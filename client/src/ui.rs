use std::collections::BTreeMap;
use std::sync::Arc;

use eframe::egui;
use tokio::runtime::Runtime;

use crate::{
    application::{
        authorization_client::AuthorizationClientService,
        ports::{RegisterUserResult, ServerProfileRepository, SessionRepository},
    },
    domain::authorization::{
        AccountKind, AuthenticationDataInfo, LoginType, LoginUserInfo, RegisterUserInfo,
        ServerCredentials, WhoAmIView,
    },
    infrastructure::http::hyper_authorization_api::HyperAuthorizationApi,
    infrastructure::persistence::sqlite_session_repository::SqliteSessionRepository,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Welcome,
    ChooseFlow,
    RegisterForm,
    WhoAmI,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum UsernameState {
    Unknown,
    Available,
    Unavailable(String),
}

pub struct ClientDesktopApplication {
    runtime: Runtime,
    session_repository: Arc<dyn SessionRepository>,
    server_profile_repository: Arc<dyn ServerProfileRepository>,
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
}

impl ClientDesktopApplication {
    pub fn new(repository: Arc<SqliteSessionRepository>) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|error| error.to_string())?;
        let session_repository: Arc<dyn SessionRepository> = repository.clone();
        let server_profile_repository: Arc<dyn ServerProfileRepository> = repository;

        let mut app = Self {
            runtime,
            session_repository,
            server_profile_repository,
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
        };

        app.reload_servers();
        Ok(app)
    }

    fn normalized_server_url(&self) -> String {
        self.server_url_input.trim().trim_end_matches('/').to_owned()
    }

    fn selected_server(&self) -> Option<&str> {
        self.servers
            .get(self.selected_server_index)
            .map(std::string::String::as_str)
    }

    fn selected_account(&self) -> Option<&ServerCredentials> {
        self.accounts_for_server.get(self.selected_account_index)
    }

    fn build_authorization_service(&self) -> AuthorizationClientService {
        let server_url = self
            .selected_server()
            .map(str::to_owned)
            .unwrap_or_else(|| self.normalized_server_url());
        let api = Arc::new(HyperAuthorizationApi::new(server_url.clone()));
        AuthorizationClientService::new(server_url, api, Arc::clone(&self.session_repository))
    }

    fn reload_servers(&mut self) {
        match self
            .runtime
            .block_on(self.server_profile_repository.list_server_credentials())
        {
            Ok(credentials) => {
                let mut servers: Vec<String> = credentials.into_iter().map(|x| x.server_url).collect();
                servers.sort();
                servers.dedup();
                self.servers = servers;

                if self.selected_server_index >= self.servers.len() {
                    self.selected_server_index = 0;
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

        match self
            .runtime
            .block_on(self.server_profile_repository.list_server_accounts(&server))
        {
            Ok(accounts) => {
                self.accounts_for_server = accounts;
                if self.selected_account_index >= self.accounts_for_server.len() {
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
        let server_url = self.new_server_url_input.trim().trim_end_matches('/').to_owned();
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
        match self.runtime.block_on(self.server_profile_repository.upsert_server_credentials(&credentials)) {
            Ok(_) => {
                self.status_message = "Account saved".to_owned();
                self.reload_accounts_for_selected_server();
                if let Some(index) = self.accounts_for_server.iter().position(|account| account.username == credentials.username) {
                    self.selected_account_index = index;
                }
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
            Ok(_) => self.load_whoami(),
            Err(error) => self.status_message = format!("Login failed: {error}"),
        }
    }

    fn load_whoami(&mut self) {
        let service = self.build_authorization_service();
        match self.runtime.block_on(service.who_am_i_from_saved_session()) {
            Ok(view) => {
                self.whoami = Some(view.clone());
                self.screen = Screen::WhoAmI;
                self.status_message = "Login successful".to_owned();
            }
            Err(error) => {
                self.status_message = format!("Failed to load whoami: {error}");
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
                }
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
            }
            Err(error) => self.status_message = format!("Logout failed: {error}"),
        }
    }

    fn render_welcome(&mut self, ui: &mut egui::Ui) {
        ui.heading("Welcome");
        ui.label("Servers");
        ui.separator();

        ui.horizontal(|ui| {
            if !self.servers.is_empty() {
                let mut changed_to_index: Option<usize> = None;
                egui::ComboBox::from_label("Server")
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
                    self.reload_accounts_for_selected_server();
                }
            }

            if ui.button("+").clicked() {
                self.show_add_server_form = true;
            }
        });

        if self.show_add_server_form {
            ui.group(|ui| {
                ui.label("Add server");
                ui.horizontal(|ui| {
                    ui.label("Server URL");
                    ui.text_edit_singleline(&mut self.new_server_url_input);
                });
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

        ui.separator();
        ui.label("My accounts on this server");
        if !self.accounts_for_server.is_empty() {
            egui::ComboBox::from_label("Account")
                .selected_text(
                    self.accounts_for_server
                        .get(self.selected_account_index)
                        .map(|account| account.username.clone())
                        .unwrap_or_else(|| "Choose account".to_owned()),
                )
                .show_ui(ui, |ui| {
                    for (index, account) in self.accounts_for_server.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_account_index == index, &account.username)
                            .clicked()
                        {
                            self.selected_account_index = index;
                            self.username_input = account.username.clone();
                            self.password_input = account.password.clone();
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Login").clicked() {
                    self.run_login_selected_account();
                }
                if ui.button("Register").clicked() {
                    self.start_registration();
                }
            });
        } else {
            ui.label("No saved accounts");
            ui.horizontal(|ui| {
                if ui.button("Login").clicked() {
                    self.status_message = "No account selected. Add or fill account first.".to_owned();
                }
                if ui.button("Register").clicked() {
                    self.start_registration();
                }
            });
            ui.separator();
            ui.label("Account credentials");
            ui.horizontal(|ui| {
                ui.label("Username");
                ui.text_edit_singleline(&mut self.username_input);
            });
            ui.horizontal(|ui| {
                ui.label("Password");
                ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));
            });
            if ui.button("Save Account").clicked() {
                self.save_selected_account_credentials();
            }
        }
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

    fn render_whoami(&mut self, ui: &mut egui::Ui) {
        ui.heading("WhoAmI");
        if let Some(view) = &self.whoami {
            ui.label(format!("User: {}", view.user_id));
            ui.label(format!("Guest: {}", view.is_guest));
            if let Some(device_id) = &view.device_id {
                ui.label(format!("Device: {device_id}"));
            }
        }

        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                self.screen = Screen::Welcome;
            }
            if ui.button("Logout").clicked() {
                self.run_logout();
            }
        });
    }
}

impl eframe::App for ClientDesktopApplication {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(context, |ui| {
            match self.screen {
                Screen::Welcome => self.render_welcome(ui),
                Screen::ChooseFlow => self.render_choose_flow(ui),
                Screen::RegisterForm => self.render_register_form(ui),
                Screen::WhoAmI => self.render_whoami(ui),
            }

            ui.separator();
            ui.label(&self.status_message);
        });
    }
}
