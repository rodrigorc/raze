use std::collections::VecDeque;

use bitflags::bitflags;

use crate::disk::{Disk, Track};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum IntStatus {
    Idle,
    Running,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorId {
    pub c: u8,
    pub h: u8,
    pub r: u8,
    pub n: u8,
}

impl SectorId {
    pub fn len(&self) -> usize {
        let n = self.n & 3;
        128 << n
    }
}

pub struct Floppy {
    cmd: Vec<u8>,
    reply: VecDeque<u8>,
    data: VecDeque<u8>,

    data_in: Option<(u8, SectorId, usize)>,

    cylinder: u8,
    int_seek_completed: IntStatus,

    read_id_idx: u16,
    disk: Disk,
    lost_read: u32,
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    struct MainReg: u8 {
        const FDD0_BUSY = 0x01;
        const FDD1_BUSY = 0x02;
        const FDD2_BUSY = 0x04;
        const FDD3_BUSY = 0x08;
        const BUSY = 0x10;
        const EXE_MODE = 0x20;
        const DIO = 0x40;
        const RQM = 0x80;
    }

    #[derive(Debug, Copy, Clone)]
    struct St0: u8 {
        const DRIVE_0 = 0x01;
        const DRIVE_1 = 0x02;
        const HEAD = 0x04;
        const NOT_READY = 0x08;
        const EQUIP_CHECK = 0x10;
        const SEEK_END = 0x20;
        const FAIL = 0x40;
        const UNKNOWN = 0x80;
    }

    #[derive(Debug, Copy, Clone)]
    pub struct St1: u8 {
        const MISSING_AM = 0x01;
        const NOT_WRITEABLE = 0x02;
        const NO_DATA = 0x04;
        const _UNUSED_0 = 0x08;
        const OVERRUN = 0x10;
        const DATA_ERROR = 0x20;
        const _UNUSED_1 = 0x40;
        const END_OF_CYLINDER = 0x80;
    }

    #[derive(Debug, Copy, Clone)]
    pub struct St2: u8 {
        const MISSING_AM_IN_DATA = 0x01;
        const BAD_CYLINDER = 0x02;
        const SCAN_NOT_SATISFIED = 0x04;
        const SCAN_EQUAL_HIT = 0x08;
        const WRONG_CYLINDER = 0x10;
        const DATA_ERROR_IN_DATA = 0x20;
        const CONTROL_MARK = 0x40;
        const _UNUSED_0 = 0x80;
    }

    #[derive(Debug, Copy, Clone)]
    struct St3: u8 {
        const DRIVE_0 = 0x01;
        const DRIVE_1 = 0x02;
        const HEAD = 0x04;
        const TWO_SIDE = 0x08;
        const TRACK_0 = 0x10;
        const READY = 0x20;
        const WRITE_PROTECTED = 0x40;
        const FAULT = 0x80;
    }

}

impl Floppy {
    pub fn new() -> Floppy {
        let disk = Disk::new_formatted();

        Floppy {
            cmd: Vec::new(),
            reply: VecDeque::new(),
            data: VecDeque::new(),
            data_in: None,
            cylinder: 0,
            int_seek_completed: IntStatus::Idle,
            read_id_idx: 0,
            disk,
            lost_read: 0,
        }
    }

    pub fn set_disk(&mut self, disk: Disk) {
        self.disk = disk;
    }

    pub fn write_cmd(&mut self, b: u8) {
        //log::info!("DAT W: {:02x}", b);
        if let Some((head, in_id, in_len)) = self.data_in.as_mut() {
            self.data.push_back(b);
            *in_len -= 1;
            if *in_len == 0 {
                let st0 = St0::FAIL;
                let st1 = St1::END_OF_CYLINDER;
                self.reply = VecDeque::from([
                    st0.bits(),
                    st1.bits(),
                    0,
                    in_id.c,
                    in_id.h,
                    in_id.r,
                    in_id.n,
                ]);
                let data = Vec::from(std::mem::take(&mut self.data));
                log::info!("{data:02x?}");

                if let Some(sector) = self
                    .disk
                    .get_track_mut(*head, self.cylinder)
                    .and_then(|track| track.get_sector_mut(in_id))
                {
                    // TODO write different length
                    sector.data = data;
                }
                self.data_in = None;
                log::info!("<<< {:02x?}", self.reply);
            }
        } else {
            self.cmd.push(b);
            self.maybe_run_cmd();
        }
    }

    pub fn read_cmd(&mut self) -> u8 {
        let r;
        if let Some(b) = self.data.pop_front() {
            r = b;
        } else if let Some(b) = self.reply.pop_front() {
            r = b;
        } else {
            log::info!("undeflow!");
            r = 0;
        }
        //log::info!("DAT R: {:02x}", r);
        self.lost_read = 0;
        r
    }

    pub fn read_status(&mut self) -> u8 {
        let mut r = MainReg::empty();

        if self.data_in.is_some() {
            r.insert(MainReg::RQM | MainReg::EXE_MODE | MainReg::BUSY);
        } else if !self.data.is_empty() {
            self.lost_read += 1;
            if self.lost_read > 2 {
                let _r = self.data.pop_front().unwrap();
                self.lost_read = 0;
                //log::info!("Lost {_r:02x}");
            }
            r.insert(MainReg::RQM | MainReg::DIO | MainReg::EXE_MODE | MainReg::BUSY);
        } else if !self.reply.is_empty() {
            r.insert(MainReg::RQM | MainReg::DIO | MainReg::BUSY);
        } else {
            r.insert(MainReg::RQM);
            if !self.cmd.is_empty() {
                r.insert(MainReg::BUSY);
            }
        }

        match self.int_seek_completed {
            IntStatus::Idle => {}
            IntStatus::Running => {
                r.insert(MainReg::FDD0_BUSY);
            }
            IntStatus::Done => {
                r.insert(MainReg::FDD0_BUSY);
                self.int_seek_completed = IntStatus::Idle;
            }
        }

        //log::info!("msr: {0:02x}  -  {0:?}", r);
        r.bits()
    }

    fn maybe_run_cmd(&mut self) {
        let len = self.cmd.len();

        match self.cmd[0] {
            0x03 => {
                if len == 3 {
                    log::info!("Specify {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            0x04 => {
                if len == 2 {
                    log::info!("Sense drive status {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    //let head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    let st3 = match drive {
                        0 => {
                            St3::READY
                                | if self.cylinder == 0 {
                                    St3::TRACK_0
                                } else {
                                    St3::empty()
                                }
                        } // | St3::TWO_SIDE,
                        _ => St3::FAULT,
                    };
                    self.reply = VecDeque::from([st3.bits() | drive]);
                    log::info!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            0x07 => {
                if len == 2 {
                    log::info!("Recalibrate {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let _drive = c1 & 0b0011;
                    self.reply = VecDeque::new();
                    self.cylinder = 0;
                    self.int_seek_completed = IntStatus::Running;
                    self.cmd.clear();
                }
            }
            0x08 => {
                if len == 1 {
                    //log::info!("Sense interrupt status {:02x?}", self.cmd);
                    let mut st0 = St0::empty();
                    if self.int_seek_completed == IntStatus::Running {
                        self.int_seek_completed = IntStatus::Done;
                        st0.insert(St0::SEEK_END);
                        //if no floppy inserted: st0.insert(St0::FAIL | St0::SEEK_END | St0::NOT_READY); //St0::SEEK_END);
                    } else {
                        st0.insert(St0::UNKNOWN);
                    }
                    self.reply = VecDeque::from([st0.bits(), self.cylinder]);
                    //log::info!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            0x0f => {
                if len == 3 {
                    log::info!("Seek {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let _drive = c1 & 0b0011;
                    self.cylinder = self.cmd[2];
                    self.int_seek_completed = IntStatus::Running;
                    self.reply = VecDeque::new();
                    self.cmd.clear();
                }
            }
            c if c & 0b0001_1111 == 0b0001_0000 => {
                if len == 1 {
                    log::info!("Version {:02x?}", self.cmd);
                    self.reply = VecDeque::from([0x80]); // PD765A
                    log::info!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            // from here on, masks
            c if c & 0b11111 == 0b00110 || c & 0b11111 == 0b01100 => {
                if len == 9 {
                    let deleted = c & 0b11111 == 0b01100;
                    let skip = c & 0x20 != 0;
                    let multitrack = c & 0x80 != 0;
                    log::info!(
                        "Read {}data {:02x?} SK={} MT={}",
                        if deleted { "deleted " } else { "" },
                        self.cmd,
                        skip,
                        multitrack
                    );
                    let c1 = self.cmd[1];
                    let head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    let c = self.cmd[2];
                    let h = self.cmd[3];
                    let r = self.cmd[4];
                    let n = self.cmd[5];
                    let eot = self.cmd[6];
                    let _gpl = self.cmd[7];
                    let _dtl = self.cmd[8];

                    if drive == 0 {
                        let id = SectorId { c, h, r, n };
                        if let Some(sector) = self
                            .disk
                            .get_track(head, self.cylinder)
                            .and_then(|track| track.get_sector(&id))
                        {
                            let mut expected_len = sector.id.len();
                            self.data = VecDeque::from(sector.data.clone());
                            log::info!("Reading {} {} = {}", self.cylinder, r, self.data.len());

                            let st0 = St0::FAIL;
                            let mut st1 = sector.st1;
                            let mut st2 = sector.st2;
                            if deleted {
                                st2.toggle(St2::CONTROL_MARK);
                            }

                            if eot > r {
                                // TODO eot < r???

                                log::info!("Multi sector read!!!");
                                //let mut cur = id;
                                let track = self.disk.get_track(head, self.cylinder).unwrap();
                                //for next_r in 0 .. track.sector_count() {
                                //let Some(next) = track.get_next_sector(&cur) else { break };
                                for next_r in r + 1..=eot {
                                    let Some(next) = track.get_sector(&SectorId {
                                        r: next_r,
                                        ..id.clone()
                                    }) else {
                                        break;
                                    };
                                    expected_len += next.id.len();
                                    self.data.extend(&next.data);
                                    log::info!("   extend {}", next.id.r);
                                    //if next.id.r == eot {
                                    //    break;
                                    //}
                                    //cur = next.id.clone();
                                }
                            }
                            if self.data.len() == expected_len {
                                st1.insert(St1::END_OF_CYLINDER);
                            } else {
                                st1.insert(St1::OVERRUN);
                            }

                            self.reply =
                                VecDeque::from([st0.bits(), st1.bits(), st2.bits(), c, h, eot, n]);
                        } else {
                            let st0 = St0::FAIL;
                            let st1 = St1::MISSING_AM;
                            self.reply =
                                VecDeque::from([st0.bits(), st1.bits(), 0, 0xff, 0xff, 0xff, 0xff]);
                        }
                    } else {
                        let st0 = St0::FAIL | St0::UNKNOWN | St0::NOT_READY;
                        self.reply = VecDeque::from([st0.bits(), 0, 0, c, h, r, n]);
                    }

                    log::info!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            c if c & 0b111111 == 0b000101 => {
                if len == 9 {
                    log::info!("Write data {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    let c = self.cmd[2];
                    let h = self.cmd[3];
                    let r = self.cmd[4];
                    let n = self.cmd[5];
                    let _eot = self.cmd[6];
                    let _gpl = self.cmd[7];
                    let _dtl = self.cmd[8];
                    if drive == 0 {
                        let sid = SectorId { c, h, r, n };
                        if let Some(_sector) = self
                            .disk
                            .get_track_mut(head, self.cylinder)
                            .and_then(|track| track.get_sector_mut(&sid))
                        {
                            let len = sid.len();
                            self.data_in = Some((head, sid, len));
                        } else {
                            let st0 = St0::FAIL;
                            let st1 = St1::MISSING_AM;
                            self.reply =
                                VecDeque::from([st0.bits(), st1.bits(), 0, 0xff, 0xff, 0xff, 0xff]);
                        }
                    } else {
                        let st0 = St0::FAIL | St0::UNKNOWN | St0::NOT_READY;
                        self.reply = VecDeque::from([st0.bits(), 0, 0, 0xff, 0xff, 0xff, 0xff]);
                    }
                    self.cmd.clear();
                }
            }
            c if c & 0b111111 == 0b001001 => {
                if len == 9 {
                    log::info!("Write deleted data *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b1001_1111 == 0b0000_0010 => {
                if len == 9 {
                    // or is it read track?
                    log::info!("Read diagnostic *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b1011_1111 == 0b0000_1010 => {
                if len == 2 {
                    log::info!("Read ID {:02x?} at cyl {}", self.cmd, self.cylinder);
                    let c1 = self.cmd[1];
                    let head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    if drive == 0 {
                        if let Some(sector) = self
                            .disk
                            .get_track(head, self.cylinder)
                            .and_then(|track| track.get_sector_by_idx(self.read_id_idx as usize))
                        {
                            let id = &sector.id;
                            self.reply = VecDeque::from([0, 0, 0, id.c, id.h, id.r, id.n]);
                        } else {
                            let st0 = St0::FAIL;
                            let st1 = St1::MISSING_AM;
                            self.reply =
                                VecDeque::from([st0.bits(), st1.bits(), 0, 0xff, 0xff, 0xff, 0xff]);
                        }
                        self.read_id_idx = self.read_id_idx.wrapping_add(1);
                    }

                    if self.reply.is_empty() {
                        let st0 = St0::UNKNOWN | St0::FAIL | St0::NOT_READY;
                        self.reply = VecDeque::from([st0.bits(), 0, 0, 0, 0, 0, 2]);
                    }
                    log::info!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            c if c & 0b1011_1111 == 0b0000_1101 => {
                if len == 6 {
                    // AKA format track
                    log::info!("Write ID {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    let n = self.cmd[2];
                    let sectors = self.cmd[3];
                    let gpl = self.cmd[4];
                    let filler = self.cmd[5];

                    if drive == 0 {
                        self.disk.set_track(
                            head,
                            self.cylinder,
                            Track::new_formatted(self.cylinder, head, n, sectors, gpl, filler),
                        );
                        self.reply = VecDeque::from([0, 0, 0, 0, 0, 0, n]);
                    } else {
                        let st0 = St0::UNKNOWN | St0::FAIL | St0::NOT_READY;
                        self.reply = VecDeque::from([st0.bits(), 0, 0, self.cylinder, head, 0, n]);
                    }
                    log::info!("<<< {:02x?}", self.reply);

                    self.cmd.clear();
                }
            }
            c if c & 0b11111 == 0b10001 => {
                if len == 9 {
                    log::info!("Scan equal *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b11111 == 0b11001 => {
                if len == 9 {
                    log::info!("Scan low or equal *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b11111 == 0b11101 => {
                if len == 9 {
                    log::info!("Scan high or equal *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c => {
                if len == 1 {
                    log::info!("Invalid {c:02x}");
                    self.cmd.clear();
                }
            }
        };
    }
}
