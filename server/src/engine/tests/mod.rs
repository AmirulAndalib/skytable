/*
 * Created on Sat Sep 23 2023
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2023, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

mod cfg {
    use crate::{
        engine::config::{
            self, CLIConfigParseReturn, ConfigEndpoint, ConfigEndpointTcp, ConfigEndpointTls,
            ConfigMode, ConfigReturn, ConfigSystem, Configuration, ParsedRawArgs,
        },
        util::test_utils::with_files,
    };

    /*
        CLI tests
    */

    fn extract_cli_args(payload: &str) -> std::collections::HashMap<String, Vec<String>> {
        extract_cli_args_raw(payload).into_config()
    }
    fn extract_cli_args_raw(
        payload: &str,
    ) -> CLIConfigParseReturn<std::collections::HashMap<String, Vec<String>>> {
        config::parse_cli_args(payload.split_ascii_whitespace().map_while(|item| {
            let mut item = item.trim();
            if item.ends_with("\n") {
                item = &item[..item.len() - 1];
            }
            if item.is_empty() {
                None
            } else {
                Some(item)
            }
        }))
        .unwrap()
    }
    #[test]
    fn parse_cli_args_simple() {
        let payload = "skyd --mode dev --endpoint tcp@localhost:2003";
        let cfg = extract_cli_args(payload);
        let expected: ParsedRawArgs = into_dict! {
            "--mode" => vec!["dev".into()],
            "--endpoint" => vec!["tcp@localhost:2003".into()]
        };
        assert_eq!(cfg, expected);
    }
    #[test]
    fn parse_cli_args_packed() {
        let payload = "skyd --mode=dev --endpoint=tcp@localhost:2003";
        let cfg = extract_cli_args(payload);
        let expected: ParsedRawArgs = into_dict! {
            "--mode" => vec!["dev".into()],
            "--endpoint" => vec!["tcp@localhost:2003".into()]
        };
        assert_eq!(cfg, expected);
    }
    #[test]
    fn parse_cli_args_multi() {
        let payload = "skyd --mode=dev --endpoint tcp@localhost:2003";
        let cfg = extract_cli_args(payload);
        let expected: ParsedRawArgs = into_dict! {
            "--mode" => vec!["dev".into()],
            "--endpoint" => vec!["tcp@localhost:2003".into()]
        };
        assert_eq!(cfg, expected);
    }
    #[test]
    fn parse_validate_cli_args() {
        with_files(
            ["__cli_args_test_private.key", "__cli_args_test_cert.pem"],
            |[pkey, cert]| {
                let payload = format!(
                    "skyd --mode=dev \
                --endpoint tcp@127.0.0.1:2003 \
                --endpoint tls@127.0.0.2:2004 \
                --service-window=600 \
                --tlskey {pkey} \
                --tlscert {cert} \
                --auth pwd"
                );
                let cfg = extract_cli_args(&payload);
                let ret = config::apply_and_validate::<config::CSCommandLine>(cfg)
                    .unwrap()
                    .into_config();
                assert_eq!(
                    ret,
                    Configuration::new(
                        ConfigEndpoint::Multi(
                            ConfigEndpointTcp::new("127.0.0.1".into(), 2003),
                            ConfigEndpointTls::new(
                                ConfigEndpointTcp::new("127.0.0.2".into(), 2004),
                                "".into(),
                                "".into()
                            )
                        ),
                        ConfigMode::Dev,
                        ConfigSystem::new(600, true)
                    )
                )
            },
        );
    }
    #[test]
    fn parse_validate_cli_args_help_and_version() {
        let pl1 = "skyd --help";
        let pl2 = "skyd --version";
        let ret1 = extract_cli_args_raw(pl1);
        let ret2 = extract_cli_args_raw(pl2);
        assert_eq!(ret1, CLIConfigParseReturn::Help);
        assert_eq!(ret2, CLIConfigParseReturn::Version);
        config::set_cli_src(vec!["skyd".into(), "--help".into()]);
        let ret3 = config::check_configuration().unwrap();
        config::set_cli_src(vec!["skyd".into(), "--version".into()]);
        let ret4 = config::check_configuration().unwrap();
        assert_eq!(
            ret3,
            ConfigReturn::HelpMessage(config::CLI_HELP.to_string())
        );
        assert_eq!(
            ret4,
            ConfigReturn::HelpMessage(format!(
                "Skytable Database Server (skyd) v{}",
                libsky::VERSION
            ))
        );
    }

    /*
        env tests
    */

    fn vars_to_args(variables: &[String]) -> ParsedRawArgs {
        variables
            .iter()
            .map(|var| {
                var.split("=")
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            })
            .map(|mut v| {
                let key = v.remove(0);
                let values = v.remove(0).split(",").map(ToString::to_string).collect();
                (key, values)
            })
            .collect()
    }
    #[test]
    fn parse_env_args_simple() {
        let variables = [
            format!("SKYDB_TLS_CERT=/var/skytable/keys/cert.pem"),
            format!("SKYDB_TLS_KEY=/var/skytable/keys/private.key"),
            format!("SKYDB_AUTH=pwd"),
            format!("SKYDB_ENDPOINTS=tcp@localhost:8080"),
            format!("SKYDB_RUN_MODE=dev"),
            format!("SKYDB_SERVICE_WINDOW=600"),
        ];
        let expected_args = vars_to_args(&variables);
        config::set_env_src(variables.into());
        let args = config::parse_env_args().unwrap().unwrap();
        assert_eq!(args, expected_args);
    }
    #[test]
    fn parse_env_args_multi() {
        let variables = [
            format!("SKYDB_TLS_CERT=/var/skytable/keys/cert.pem"),
            format!("SKYDB_TLS_KEY=/var/skytable/keys/private.key"),
            format!("SKYDB_AUTH=pwd"),
            format!("SKYDB_ENDPOINTS=tcp@localhost:8080,tls@localhost:8081"),
            format!("SKYDB_RUN_MODE=dev"),
            format!("SKYDB_SERVICE_WINDOW=600"),
        ];
        let expected_args = vars_to_args(&variables);
        config::set_env_src(variables.into());
        let args = config::parse_env_args().unwrap().unwrap();
        assert_eq!(args, expected_args);
    }
    #[test]
    fn parse_validate_env_args() {
        with_files(
            ["__env_args_test_cert.pem", "__env_args_test_private.key"],
            |[cert, key]| {
                let variables = [
                    format!("SKYDB_TLS_CERT={cert}"),
                    format!("SKYDB_TLS_KEY={key}"),
                    format!("SKYDB_AUTH=pwd"),
                    format!("SKYDB_ENDPOINTS=tcp@localhost:8080,tls@localhost:8081"),
                    format!("SKYDB_RUN_MODE=dev"),
                    format!("SKYDB_SERVICE_WINDOW=600"),
                ];
                config::set_env_src(variables.into());
                let cfg = config::check_configuration().unwrap().into_config();
                assert_eq!(
                    cfg,
                    Configuration::new(
                        ConfigEndpoint::Multi(
                            ConfigEndpointTcp::new("localhost".into(), 8080),
                            ConfigEndpointTls::new(
                                ConfigEndpointTcp::new("localhost".into(), 8081),
                                "".into(),
                                "".into()
                            )
                        ),
                        ConfigMode::Dev,
                        ConfigSystem::new(600, true)
                    )
                )
            },
        );
    }
}
