use super::*;
use crate::protools::ptsl::CommandId;
use std::fmt;

#[derive(Debug, Default)]
pub struct Timecode {
    hr: i64,
    min: i64,
    sec: i64,
    fr: f64,
    fps: i64,
}
impl fmt::Display for Timecode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}:{:02}",
            self.hr, self.min, self.sec, self.fr
        )
    }
}
impl Timecode {
    pub async fn from_hmsf(
        hr: i64,
        min: i64,
        sec: i64,
        fr: f64,
        pt: &mut ProtoolsSession,
    ) -> R<Self> {
        let fps = pt.get_frames_per_second().await?;
        let mut s = Self {
            hr,
            min,
            sec,
            fr,
            fps,
        };
        s.normalize();
        Ok(s)
    }
    pub async fn from_string(tc: &str, pt: &mut ProtoolsSession) -> R<Self> {
        let fps = pt.get_frames_per_second().await?;
        let v: Vec<f64> = tc
            .split(":")
            .map(|n| n.parse::<f64>().unwrap_or(0.0))
            .collect();
        let mut s = Self {
            hr: v.first().copied().unwrap_or(0.0) as i64,
            min: v.get(1).copied().unwrap_or(0.0) as i64,
            sec: v.get(2).copied().unwrap_or(0.0) as i64,
            fr: v.get(3).copied().unwrap_or(0.0),
            fps,
        };
        s.normalize();
        Ok(s)
    }
    fn normalize(&mut self) {
        while self.fr >= self.fps as f64 {
            self.sec += 1;
            self.fr -= self.fps as f64;
        }
        while self.fr < 0.0 {
            self.sec -= 1;
            self.fr += self.fps as f64;
        }

        while self.sec >= 60 {
            self.min += 1;
            self.sec -= 60;
        }
        while self.sec < 0 {
            self.min -= 1;
            self.sec += 60;
        }

        while self.min >= 60 {
            self.hr += 1;
            self.min -= 60;
        }
        while self.min < 0 {
            self.hr -= 1;
            self.min += 60;
        }

        if self.hr < 0 {
            self.hr = 0;
            self.min = 0;
            self.sec = 0;
            self.fr = 0.0;
        }
    }
    pub fn snap_to_grid(&mut self) {
        self.fr = self.fr.round();
    }

    pub fn add_hmsf(&mut self, hr: i64, min: i64, sec: i64, fr: f64) {
        self.hr += hr;
        self.min += min;
        self.sec += sec;
        self.fr += fr;
        self.normalize();
    }

    pub fn sub_hmsf(&mut self, hr: i64, min: i64, sec: i64, fr: f64) {
        self.hr -= hr;
        self.min -= min;
        self.sec -= sec;
        self.fr -= fr;
        self.normalize();
    }
}
#[derive(Debug, Default)]
pub struct PtSelectionTimecode {
    play_start_marker_time: String,
    in_time: String,
    out_time: String,
    pre_roll_start_time: String,
    post_roll_stop_time: String,
    pre_roll_enabled: bool,
    post_roll_enabled: bool,
}

impl PtSelectionTimecode {
    pub async fn new(pt: &mut ProtoolsSession) -> R<Self> {
        let mut s = Self::default();
        s.get(pt).await?;
        Ok(s)
    }
    pub async fn get(&mut self, pt: &mut ProtoolsSession) -> R<()> {
        log::info!("Requesting timeline selection...");

        // Pro Tools expects the enum as a STRING in JSON, not an integer!
        // Use raw JSON instead of the protobuf struct
        let response: serde_json::Value = pt
            .cmd(
                CommandId::GetTimelineSelection,
                serde_json::json!({
                    "location_type": "TLType_TimeCode"
                }),
            )
            .await?;

        log::info!("Timeline selection response: {:?}", response);
        self.play_start_marker_time = response["play_start_marker_time"]
            .as_str()
            .unwrap_or("00:00:00:00")
            .to_string();

        self.in_time = response["in_time"]
            .as_str()
            .unwrap_or("00:00:00:00")
            .to_string();
        self.out_time = response["out_time"]
            .as_str()
            .unwrap_or("00:00:00:00")
            .to_string();

        self.pre_roll_start_time = response["pre_roll_start_time"]
            .as_str()
            .unwrap_or("00:00:00:00")
            .to_string();
        self.post_roll_stop_time = response["post_roll_stop_time"]
            .as_str()
            .unwrap_or("00:00:00:00")
            .to_string();
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
    pub async fn set(&mut self, pt: &mut ProtoolsSession) -> R<()> {
        let response: serde_json::Value = pt
            .cmd(
                CommandId::SetTimelineSelection,
                serde_json::json!({
                    "location_type": "TLType_TimeCode",
                    "play_start_marker_time": self.play_start_marker_time,
                    "in_time": self.in_time,
                          "out_time": self.out_time,
                          "pre_roll_start_time": self.pre_roll_start_time,
                          "post_roll_stop_time": self.post_roll_stop_time,
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
        in_time: &Timecode,
        out_time: &Timecode,
    ) -> R<()> {
        self.pre_roll_start_time = in_time.to_string();
        self.post_roll_stop_time = out_time.to_string();
        self.in_time = in_time.to_string();
        self.out_time = out_time.to_string();
        self.set(pt).await?;
        Ok(())
    }
    pub async fn get_io(&self, pt: &mut ProtoolsSession) -> R<(Timecode, Timecode)> {
        let i = Timecode::from_string(&self.in_time, pt).await?;
        let o = Timecode::from_string(&self.out_time, pt).await?;
        Ok((i, o))
    }
    // pub async fn add(&mut self, pt: &mut ProtoolsSession, value: &str) -> R<()> {
    //     self.in_time += value;
    //     self.out_time += value;
    //     self.pre_roll_start_time += value;
    //     self.post_roll_stop_time += value;
    //     self.set(pt).await?;
    //     Ok(())
    // }
}
