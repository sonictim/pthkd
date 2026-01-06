use super::ptsl;
use anyhow::{Context, Result};
use ptsl::*;
use tonic::Request as TonicRequest;
use tonic::transport::Channel;

pub struct ProtoolsSession {
    client: ptsl::ptsl_client::PtslClient<Channel>,
    session_id: String,
}

impl ProtoolsSession {
    pub async fn new() -> Result<Self> {
        println!("Connecting to Pro Tools...");

        let channel = Channel::from_static("http://localhost:31416")
            .connect()
            .await
            .context("Failed to connect to Pro Tools - is it running?")?;

        let mut s = Self {
            client: ptsl::ptsl_client::PtslClient::new(channel),
            session_id: String::new(),
        };

        let session_data: ptsl::RegisterConnectionResponseBody = s
            .cmd(
                CommandId::RegisterConnection,
                ptsl::RegisterConnectionRequestBody {
                    company_name: "Feral Frequencies".to_string(),
                    application_name: "pt-cli".to_string(),
                },
            )
            .await?;

        s.session_id = session_data.session_id;

        println!("\n✅ Connected! Session ID: {}", s.session_id);

        Ok(s)
    }

    pub async fn cmd<TReq, TResp>(&mut self, command_id: CommandId, body: TReq) -> Result<TResp>
    where
        TReq: serde::Serialize,
        TResp: serde::de::DeserializeOwned,
    {
        let body_json = serde_json::to_string(&body)?;
        eprintln!("Request body JSON: {}", body_json);

        let request = Request {
            header: Some(RequestHeader {
                task_id: String::new(),
                command: command_id as i32,
                version: 2025,
                session_id: self.session_id.clone(),
                version_minor: 10,
                version_revision: 0,
                versioned_request_header_json: String::new(),
            }),
            request_body_json: body_json,
        };

        let response = self
            .client
            .send_grpc_request(TonicRequest::new(request))
            .await?
            .into_inner();

        // Check for errors
        if !response.response_error_json.is_empty() {
            eprintln!("Pro Tools Error: {}", response.response_error_json);
        }

        if response.response_body_json.is_empty() {
            Ok(serde_json::from_str("{}")?)
        } else {
            Ok(serde_json::from_str(&response.response_body_json)?)
        }
    }

    pub async fn get_session_name(&mut self) -> Result<String> {
        let response: serde_json::Value = self
            .cmd(CommandId::GetSessionName, serde_json::json!({}))
            .await?;

        Ok(response["session_name"].to_string())
    }
    pub async fn get_session_path(&mut self) -> anyhow::Result<std::path::PathBuf> {
        let response: serde_json::Value = self
            .cmd(CommandId::GetSessionPath, serde_json::json!({}))
            .await?;

        let path = response
            .get("session_path")
            .and_then(|v| v.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Unable to obtain path"))?;

        Ok(std::path::PathBuf::from(path))
    }
    pub async fn save_session(&mut self) -> Result<()> {
        let _response: serde_json::Value = self
            .cmd(CommandId::SaveSession, serde_json::json!({}))
            .await?;

        println!("Session Saved");
        Ok(())
    }
    pub async fn save_session_as(&mut self, name: &str, location: &str) -> Result<()> {
        let _: serde_json::Value = self
            .cmd(
                CommandId::SaveSessionAs,
                ptsl::SaveSessionAsRequestBody {
                    session_name: name.into(),
                    session_location: location.into(),
                },
            )
            .await?;

        println!("Session Saved");
        Ok(())
    }
    pub async fn clear(&mut self) -> Result<()> {
        let _response: serde_json::Value =
            self.cmd(CommandId::Clear, serde_json::json!({})).await?;

        println!("Clear");
        Ok(())
    }
    pub async fn cut(&mut self) -> Result<()> {
        let _response: serde_json::Value = self.cmd(CommandId::Cut, serde_json::json!({})).await?;

        println!("Cut");
        Ok(())
    }
    pub async fn copy(&mut self) -> Result<()> {
        let _response: serde_json::Value = self.cmd(CommandId::Copy, serde_json::json!({})).await?;

        println!("Copy");
        Ok(())
    }
    pub async fn paste(&mut self) -> Result<()> {
        let _response: serde_json::Value =
            self.cmd(CommandId::Paste, serde_json::json!({})).await?;

        println!("Paste");
        Ok(())
    }
    pub async fn paste_to_fill_selection(&mut self) -> Result<()> {
        let _response: serde_json::Value = self
            .cmd(
                CommandId::PasteSpecial,
                serde_json::json!({
                    "paste_special_option": "Repeat_To_Fill_Selection"
                }),
            )
            .await?;

        println!("Paste to Fill");
        Ok(())
    }
    pub async fn get_all_tracks(&mut self) -> Option<Vec<serde_json::Value>> {
        println!("\nFetching track list...");
        let response: serde_json::Value = self
            .cmd(
                CommandId::GetTrackList,
                ptsl::GetTrackListRequestBody {
                    pagination_request: Some(ptsl::PaginationRequest {
                        limit: 0,
                        offset: 0,
                    }),
                    track_filter_list: vec![ptsl::TrackListInvertibleFilter {
                        filter: 1, // TLFilter_All
                        is_inverted: false,
                    }],
                    is_filter_list_additive: true,
                    ..Default::default() // Fill in any other fields with defaults
                },
            )
            .await
            .ok()?;

        response["track_list"].as_array().cloned()
    }

    pub async fn get_all_markers(&mut self) -> Option<Vec<serde_json::Value>> {
        println!("\nFetching marker list...");
        let response: serde_json::Value = self
            .cmd(
                CommandId::GetMemoryLocations,
                serde_json::json!({

                "pagination_request": {
                    "limit": 0,
                    "offset": 0,
                },

                      }),
            )
            .await
            .ok()?;

        response["memory_locations"].as_array().cloned()
    }
    pub async fn get_used_marker_ruler_names(&mut self) -> Option<Vec<String>> {
        let markers = self.get_all_markers().await?;

        let mut ruler_names = Vec::new();

        for marker in markers {
            // Check if marker is on a named ruler
            if marker["location"].as_str() == Some("MarkerLocation_NamedRuler")
                && let Some(ruler_name) = marker["track_name"].as_str()
                && !ruler_name.is_empty()
            {
                let ruler_name = ruler_name.to_string();
                // Only add if not already in the list (preserve order)
                if !ruler_names.contains(&ruler_name) {
                    ruler_names.push(ruler_name);
                }
            }
        }

        Some(ruler_names)
    }
    pub async fn solo_tracks(&mut self, tracks: Vec<String>, state: bool) -> Result<()> {
        if !tracks.is_empty() {
            let _: serde_json::Value = self
                .cmd(
                    CommandId::SetTrackSoloState,
                    ptsl::SetTrackSoloStateRequestBody {
                        track_names: tracks,
                        enabled: state,
                    },
                )
                .await?;
        }
        Ok(())
    }
    pub async fn get_samplerate(&mut self) -> Result<i64> {
        let response: serde_json::Value = self
            .cmd(CommandId::GetSessionSampleRate, serde_json::json!({}))
            .await?;

        let rate = response["sample_rate"]
            .as_str()
            .unwrap()
            .replace("SR_", "")
            .parse::<i64>()
            .unwrap_or(48000);
        println!("Samplerate is: {}", rate);
        Ok(rate)
    }
    pub async fn get_frames_per_second(&mut self) -> Result<i64> {
        let response: serde_json::Value = self
            .cmd(CommandId::GetSessionTimeCodeRate, serde_json::json!({}))
            .await?;

        let rate = response["current_setting"]
            .as_str()
            .unwrap()
            .replace("STCR_Fps", "")
            .replace("Drop", "")
            .replace("2997", "30")
            .replace("23976", "24")
            .replace("47952", "48")
            .replace("5994", "60")
            .replace("11988", "120")
            .parse::<i64>()
            .unwrap_or(24);
        println!("Timecode FPS is: {}", rate);
        Ok(rate)
    }

    pub async fn get_edit_mode(&mut self) -> Result<String> {
        let response: serde_json::Value = self
            .cmd(CommandId::GetEditMode, serde_json::json!({}))
            .await?;

        let mode = response["current_setting"].as_str().unwrap();
        println!("current mode is: {}", mode);
        Ok(mode.to_string())
    }

    pub async fn set_edit_mode(&mut self, mode: &str) -> Result<()> {
        let _: serde_json::Value = self
            .cmd(
                CommandId::SetEditMode,
                serde_json::json!({
                    "edit_mode": mode
                }),
            )
            .await?;
        Ok(())
    }
    pub async fn get_edit_tool(&mut self) -> Result<String> {
        let response: serde_json::Value = self
            .cmd(CommandId::GetEditTool, serde_json::json!({}))
            .await?;

        let mode = response["current_setting"].as_str().unwrap();
        println!("current tool is: {}", mode);
        Ok(mode.to_string())
    }

    pub async fn set_edit_tool(&mut self, tool: &str) -> Result<()> {
        let _: serde_json::Value = self
            .cmd(
                CommandId::SetEditTool,
                serde_json::json!({
                    "edit_tool": tool
                }),
            )
            .await?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn edit_marker(
        &mut self,
        number: u32,
        name: &str,
        start_time: i64,
        end_time: i64,
        destination: MarkerLocation,
        destination_name: &str,
        color: &str,
    ) -> Result<()> {
        let color_index = match color.to_lowercase().as_str() {
            "dark purple" => 1,
            "purple" => 2,
            "pink" => 3,
            "magenta" => 4,
            "red" => 5,
            "orange" => 6,
            "dark yellow" => 7,
            "yellow" => 8,
            "light green" => 9,
            "green" => 10,
            "light blue" => 11,
            "blue" => 12,
            "dark blue" => 13,
            "white" => 14,
            "grey" => 15,
            "black" => 16,
            _ => 1,
        };
        let _: serde_json::Value = self
            .cmd(
                CommandId::EditMemoryLocation,
                serde_json::json!({

                "number": number,
                "name": name,
                "start_time": start_time.to_string(),
                "end_time": end_time.to_string(),
                "time_properties": "TProperties_Marker",
                "reference": "MLReference_FollowTrackTimebase",
                "general_properties": {
                    "zoom_settings": false,
                    "pre_post_roll_times": false,
                    "track_visibility": false,
                    "track_heights": false,
                    "group_enables": false,
                    "window_configuration": false,
                    "window_configuration_index": 1,
                    "venue_snapshot_index": 1
                },
                "comments": "comments",
                "color_index": color_index,
                "location": destination.as_str(),
                "track_name": destination_name

                      }),
            )
            .await?;
        Ok(())
    }

    pub async fn go_to_next_marker(&mut self, location: &str, reverse: bool) -> Result<()> {
        let mut selection = PtSelectionSamples::new(self).await?;
        let (selection_time, _) = selection.get_io();
        let markers = self.get_all_markers().await.unwrap_or(Vec::new());

        let mut next_marker_time: Option<i64> = None;

        for marker in markers {
            let marker_location = marker["track_name"].as_str().unwrap_or("");
            if !location.is_empty() && location != marker_location {
                continue;
            };
            let marker_time_str = marker["start_time"].as_str().unwrap_or("0");
            let marker_time = marker_time_str.parse::<i64>().unwrap_or(0);

            match reverse {
                true => {
                    if marker_time < selection_time {
                        match next_marker_time {
                            None => next_marker_time = Some(marker_time),
                            Some(current_next) => {
                                if marker_time > current_next {
                                    next_marker_time = Some(marker_time);
                                }
                            }
                        }
                    }
                }
                false => {
                    if marker_time > selection_time {
                        match next_marker_time {
                            None => next_marker_time = Some(marker_time),
                            Some(current_next) => {
                                if marker_time < current_next {
                                    next_marker_time = Some(marker_time);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(time) = next_marker_time {
            selection.set_io(self, time, time).await?;
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct PtSelectionSamples {
    play_start_marker_time: i64,
    in_time: i64,
    out_time: i64,
    pre_roll_start_time: i64,
    post_roll_stop_time: i64,
    pre_roll_enabled: bool,
    post_roll_enabled: bool,
}

impl PtSelectionSamples {
    pub async fn new(pt: &mut ProtoolsSession) -> Result<Self> {
        let mut s = Self::default();
        s.get(pt).await?;
        Ok(s)
    }
    pub async fn get(&mut self, pt: &mut ProtoolsSession) -> Result<()> {
        log::info!("Requesting timeline selection...");

        // Pro Tools expects the enum as a STRING in JSON, not an integer!
        // Use raw JSON instead of the protobuf struct
        let response: serde_json::Value = pt
            .cmd(
                CommandId::GetTimelineSelection,
                serde_json::json!({
                    "location_type": "TLType_Samples"
                }),
            )
            .await?;

        log::info!("Timeline selection response: {:?}", response);
        self.play_start_marker_time = response["play_start_marker_time"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        self.in_time = response["in_time"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        self.out_time = response["out_time"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        self.pre_roll_start_time = response["pre_roll_start_time"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        self.post_roll_stop_time = response["post_roll_stop_time"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        self.pre_roll_enabled = response["pre_roll_start_time"]
            .as_str()
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        self.post_roll_enabled = response["post_roll_stop_time"]
            .as_str()
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        println!("{:?}", self);
        Ok(())
    }
    async fn set(&mut self, pt: &mut ProtoolsSession) -> Result<()> {
        let response: serde_json::Value = pt
            .cmd(
                CommandId::SetTimelineSelection,
                serde_json::json!({
                    "location_type": "TLType_Samples",
                    "play_start_marker_time": self.play_start_marker_time.to_string(),
                    "in_time": self.in_time.to_string(),
                          "out_time": self.out_time.to_string(),
                          "pre_roll_start_time": self.pre_roll_start_time.to_string(),
                          "post_roll_stop_time": self.post_roll_stop_time.to_string(),
                          "pre_roll_enabled": self.pre_roll_enabled.to_string(),
                          "post_roll_enable": self.post_roll_enabled.to_string(),

                }),
            )
            .await?;
        println!("set selection response: {:?}", response);
        Ok(())
    }
    pub async fn set_io(
        &mut self,
        pt: &mut ProtoolsSession,
        in_time: i64,
        out_time: i64,
    ) -> Result<()> {
        self.pre_roll_start_time += in_time - self.in_time;
        self.post_roll_stop_time += out_time - self.out_time;
        self.in_time = in_time;
        self.out_time = out_time;
        self.set(pt).await?;
        Ok(())
    }
    pub fn get_io(&self) -> (i64, i64) {
        (self.in_time, self.out_time)
    }
    pub async fn slide(&mut self, pt: &mut ProtoolsSession, value: i64) -> Result<()> {
        self.in_time += value;
        self.out_time += value;
        self.pre_roll_start_time += value;
        self.post_roll_stop_time += value;
        self.set(pt).await?;
        Ok(())
    }
}

pub enum MarkerLocation {
    Track,
    NamedRuler,
    MainRuler,
}

impl MarkerLocation {
    pub fn as_str(&self) -> &str {
        match self {
            MarkerLocation::Track => "MarkerLocation_Track",
            MarkerLocation::NamedRuler => "MarkerLocation_NamedRuler",
            MarkerLocation::MainRuler => "MarkerLocation_MainRuler",
        }
    }
}
pub async fn save_protools_session() -> Result<()> {
    let mut pt = ProtoolsSession::new().await?;
    pt.save_session().await?;
    Ok(())
}
