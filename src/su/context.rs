use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use crate::common::resolve::{is_valid_executable, resolve_current_user};
use crate::common::{error::Error, Environment};
use crate::exec::RunOptions;
use crate::log::user_warn;
use crate::system::{Group, Process, User};

use super::cli::SuOptions;

const VALID_LOGIN_SHELLS_LIST: &str = "/etc/shells";
const FALLBACK_LOGIN_SHELL: &str = "/bin/sh";

const PATH_MAILDIR: &str = env!("PATH_MAILDIR");
const PATH_DEFAULT: &str = env!("SU_PATH_DEFAULT");
const PATH_DEFAULT_ROOT: &str = env!("SU_PATH_DEFAULT_ROOT");

#[derive(Debug)]
pub(crate) struct SuContext {
    command: PathBuf,
    arguments: Vec<String>,
    options: SuOptions,
    pub(crate) environment: Environment,
    user: User,
    requesting_user: User,
    group: Group,
    pub(crate) process: Process,
}

/// check that a shell is not restricted / exists in /etc/shells
fn is_restricted(shell: &Path) -> bool {
    if let Some(pattern) = shell.as_os_str().to_str() {
        if let Ok(contents) = fs::read_to_string(VALID_LOGIN_SHELLS_LIST) {
            return !contents.lines().any(|l| l == pattern);
        } else {
            return FALLBACK_LOGIN_SHELL != pattern;
        }
    }

    true
}

impl SuContext {
    pub(crate) fn from_env(options: SuOptions) -> Result<SuContext, Error> {
        let process = crate::system::Process::new();

        // resolve environment, reset if this is a login
        let mut environment = if options.login {
            Environment::default()
        } else {
            env::vars_os().collect::<Environment>()
        };

        // Don't reset the environment variables specified in the
        // comma-separated list when clearing the environment for
        // --login. The whitelist is ignored for the environment
        // variables HOME, SHELL, USER, LOGNAME, and PATH.
        if options.login {
            if let Some(value) = env::var_os("TERM") {
                environment.insert("TERM".into(), value);
            }

            for name in options.whitelist_environment.iter() {
                if let Some(value) = env::var_os(name) {
                    environment.insert(name.into(), value);
                }
            }
        }

        let requesting_user = resolve_current_user()?;

        // resolve target user
        let mut user = User::from_name(&options.user)?
            .ok_or_else(|| Error::UserNotFound(options.user.clone()))?;

        // check the current user is root
        let is_current_root = User::real_uid() == 0;
        let is_target_root = options.user == "root";

        // only root can set a (additional) group
        if !is_current_root && (!options.supp_group.is_empty() || !options.group.is_empty()) {
            return Err(Error::Options(
                "only root can specify alternative groups".to_owned(),
            ));
        }

        // resolve target group
        let mut group =
            Group::from_gid(user.gid)?.ok_or_else(|| Error::GroupNotFound(user.gid.to_string()))?;

        if !options.supp_group.is_empty() || !options.group.is_empty() {
            user.groups.clear();
        }

        for group_name in options.group.iter() {
            let primary_group = Group::from_name(group_name)?
                .ok_or_else(|| Error::GroupNotFound(group_name.to_owned()))?;

            // last argument is the primary group
            group = primary_group.clone();
            user.groups.push(primary_group.gid);
        }

        // add additional group if current user is root
        for (index, group_name) in options.supp_group.iter().enumerate() {
            let supp_group = Group::from_name(group_name)?
                .ok_or_else(|| Error::GroupNotFound(group_name.to_owned()))?;

            // set primary group if none was provided
            if index == 0 && options.group.is_empty() {
                group = supp_group.clone();
            }

            user.groups.push(supp_group.gid);
        }

        // the shell specified with --shell
        // the shell specified in the environment variable SHELL, if the --preserve-environment option is used
        // the shell listed in the passwd entry of the target user
        let user_shell = user.shell.clone();

        let mut command = options
            .shell
            .as_ref()
            .cloned()
            .or_else(|| {
                if options.preserve_environment && is_current_root {
                    environment.get(&OsString::from("SHELL")).map(|v| v.into())
                } else {
                    None
                }
            })
            .unwrap_or(user_shell.clone());

        // If the target user has a restricted shell (i.e. the shell field of
        // this user's entry in /etc/passwd is not listed in /etc/shells),
        // then the --shell option or the $SHELL environment variable won't be
        // taken into account, unless su is called by root.
        if is_restricted(user_shell.as_path()) && !is_current_root {
            user_warn!(
                "using restricted shell {}",
                user_shell.as_os_str().to_string_lossy()
            );
            command = user_shell;
        }

        if !command.exists() {
            return Err(Error::CommandNotFound(command));
        }

        if !is_valid_executable(&command) {
            return Err(Error::InvalidCommand(command));
        }

        // pass command to shell
        let arguments = if let Some(command) = &options.command {
            vec!["-c".to_owned(), command.to_owned()]
        } else {
            options.arguments.clone()
        };

        if options.login {
            environment.insert(
                "PATH".into(),
                if is_target_root {
                    PATH_DEFAULT_ROOT
                } else {
                    PATH_DEFAULT
                }
                .into(),
            );
        }

        if !options.preserve_environment {
            // extend environment with fixed variables
            environment.insert("HOME".into(), user.home.clone().into_os_string());
            environment.insert("SHELL".into(), command.clone().into());
            environment.insert(
                "MAIL".into(),
                format!("{PATH_MAILDIR}/{}", user.name).into(),
            );

            if !is_target_root || options.login {
                environment.insert("USER".into(), options.user.clone().into());
                environment.insert("LOGNAME".into(), options.user.clone().into());
            }
        }

        Ok(SuContext {
            command,
            arguments,
            options,
            environment,
            user,
            requesting_user,
            group,
            process,
        })
    }
}

impl RunOptions for SuContext {
    fn command(&self) -> io::Result<&PathBuf> {
        Ok(&self.command)
    }

    fn arguments(&self) -> &Vec<String> {
        &self.arguments
    }

    fn chdir(&self) -> Option<&std::path::PathBuf> {
        None
    }

    fn is_login(&self) -> bool {
        self.options.login
    }

    fn user(&self) -> &crate::system::User {
        &self.user
    }

    fn requesting_user(&self) -> &User {
        &self.requesting_user
    }

    fn group(&self) -> &crate::system::Group {
        &self.group
    }

    fn pid(&self) -> i32 {
        self.process.pid
    }

    fn use_pty(&self) -> bool {
        self.options.pty
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{common::Error, su::cli::SuOptions};

    use super::SuContext;

    fn get_options(args: &[&str]) -> SuOptions {
        let mut args = args.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        args.insert(0, "/bin/su".to_string());
        SuOptions::parse_arguments(args).unwrap()
    }

    #[test]
    fn su_to_root() {
        let options = get_options(&["root"]);
        let context = SuContext::from_env(options).unwrap();

        assert_eq!(context.user.name, "root");
    }

    #[test]
    fn group_as_non_root() {
        let options = get_options(&["-g", "root"]);
        let result = SuContext::from_env(options);
        let expected = Error::Options("only root can specify alternative groups".to_owned());

        assert!(result.is_err());
        assert_eq!(format!("{}", result.err().unwrap()), format!("{expected}"));
    }

    #[test]
    fn invalid_shell() {
        let options = get_options(&["-s", "/not/a/shell"]);
        let result = SuContext::from_env(options);
        let expected = Error::CommandNotFound(PathBuf::from("/not/a/shell"));

        assert!(result.is_err());
        assert_eq!(format!("{}", result.err().unwrap()), format!("{expected}"));
    }
}
