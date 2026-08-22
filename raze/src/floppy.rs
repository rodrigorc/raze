/*!
 * Emulates a floppy drive controller uPD765A.
 */

use std::borrow::BorrowMut;
use std::io::Cursor;

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
    /// The command as being received.
    cmd: Vec<u8>,
    /// Reply to be read.
    reply: Cursor<Vec<u8>>,
    /// Data to be read.
    data: Cursor<Vec<u8>>,

    /// If a write command is in progress, this has (cmd[1], SectorId, expected_len, data).
    data_in: Option<(u8, SectorId, usize, Vec<u8>)>,

    /// Is motor on?
    motor: bool,
    /// The current cylinder the head is over.
    cylinder: u8,
    /// The status of the last seek command.
    int_seek_completed: IntStatus,

    /// The index of the sector read by the next "Read ID" command.
    /// Some programs issue repeating "Read ID" to explore the track, and we don't want
    /// to read the same sector every time.
    read_id_idx: u16,

    /// The inserted disk, if any.
    disk: Option<Disk>,

    /// Counts how many statuses have been read when there are pending data bytes.
    /// If there is pending data to be read, but the CPU reads the status instead a few times
    /// the data byte is lost, overriden by the next one.
    lost_read: u32,
}

/// Extension methods for Cursor<Vec<u8>>.
trait CursorExt: BorrowMut<Cursor<Vec<u8>>> {
    #[inline]
    fn next_byte(&mut self) -> Option<u8> {
        // Could use Read::read_exact(), but that would probably be slower
        let this = self.borrow_mut();
        let p = this.position();
        let b = this.get_ref().get(p as usize).copied()?;
        this.set_position(p + 1);
        Some(b)
    }
    #[inline]
    fn is_empty(&self) -> bool {
        // Could use BufRead::has_data_left() but that is unstable.
        let this = self.borrow();
        this.position() as usize >= this.get_ref().len()
    }

    #[inline]
    fn set(&mut self, data: &[u8]) {
        let this = self.borrow_mut();
        this.set_position(0);
        let v = this.get_mut();
        v.clear();
        v.extend(data);
    }
}

//impl CursorExt for Cursor<Vec<u8>> { }
impl<T: BorrowMut<Cursor<Vec<u8>>>> CursorExt for T {}

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

impl St0 {
    fn from_c1(c1: u8) -> St0 {
        St0::from_bits_retain(c1 & 0x7)
    }
}

impl St3 {
    fn from_c1(c1: u8) -> St3 {
        St3::from_bits_retain(c1 & 0x7)
    }
}

impl Floppy {
    pub fn new() -> Floppy {
        Floppy {
            cmd: Vec::new(),
            reply: Cursor::default(),
            data: Cursor::default(),
            data_in: None,
            motor: false,
            cylinder: 0,
            int_seek_completed: IntStatus::Idle,
            read_id_idx: 0,
            disk: None,
            lost_read: 0,
        }
    }

    pub fn set_motor(&mut self, motor: bool) {
        self.motor = motor;
    }

    pub fn motor(&self) -> bool {
        self.motor
    }

    pub fn current_track(&self) -> u8 {
        self.cylinder
    }

    pub fn disk(&self) -> Option<&Disk> {
        self.disk.as_ref()
    }

    pub fn set_disk(&mut self, disk: Option<Disk>) {
        self.disk = disk;
    }

    pub fn write_cmd(&mut self, b: u8) {
        //log::debug!("DAT W: {:02x}", b);
        // Is in the middle of a "Write" command?
        if let Some((c1, in_id, in_len, data)) = self.data_in.as_mut() {
            let c1 = *c1;
            data.push(b);
            *in_len -= 1;
            // Write is complete, commit to disk
            if *in_len == 0 {
                let st0 = St0::from_c1(c1) | St0::FAIL;
                let st1 = St1::END_OF_CYLINDER;
                self.reply.set(&[
                    st0.bits(),
                    st1.bits(),
                    0,
                    in_id.c,
                    in_id.h,
                    in_id.r,
                    in_id.n,
                ]);

                //log::debug!("{data:02x?}");

                let head = (c1 & 0b0100 != 0) as u8;
                if let Some(sector) = self
                    .disk
                    .as_mut()
                    .and_then(|d| d.get_track_mut(head, self.cylinder))
                    .and_then(|track| track.get_sector_mut(in_id))
                {
                    // TODO write different length
                    sector.data = std::mem::take(data);
                }
                self.data_in = None;
                //log::debug!("<<< {:02x?}", self.reply);
            }
        } else {
            self.cmd.push(b);
            self.maybe_run_cmd();
        }
    }

    pub fn read_cmd(&mut self) -> u8 {
        let r;
        if let Some(b) = self.data.next_byte() {
            r = b;
        } else if let Some(b) = self.reply.next_byte() {
            r = b;
        } else {
            //log::info!("undeflow!");
            r = 0;
        }
        //log::debug!("DAT R: {:02x}", r);
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
                let _r = self.data.next_byte();
                self.lost_read = 0;
                //log::debug!("Lost {_r:02x}");
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

        //log::debug!("msr: {0:02x}  -  {0:?}", r);
        r.bits()
    }

    fn maybe_run_cmd(&mut self) {
        let len = self.cmd.len();

        match self.cmd[0] {
            0x03 => {
                if len == 3 {
                    //log::debug!("Specify {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            0x04 => {
                if len == 2 {
                    //log::debug!("Sense drive status {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let _head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    let st3 = match (drive, &self.disk) {
                        (0, Some(_)) => {
                            St3::from_c1(c1)
                                | St3::READY
                                | St3::TWO_SIDE
                                | if self.cylinder == 0 {
                                    St3::TRACK_0
                                } else {
                                    St3::empty()
                                }
                        }
                        _ => St3::FAULT,
                    };
                    self.reply.set(&[st3.bits() | drive]);
                    //log::debug!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            0x07 => {
                if len == 2 {
                    //log::debug!("Recalibrate {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let _head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    if drive == 0 {
                        self.int_seek_completed = IntStatus::Running;
                        self.cylinder = 0;
                        self.reply.set(&[]);
                    }
                    self.cmd.clear();
                }
            }
            0x08 => {
                if len == 1 {
                    //log::debug!("Sense interrupt status {:02x?}", self.cmd);
                    let mut st0 = St0::empty();
                    if self.int_seek_completed == IntStatus::Running {
                        self.int_seek_completed = IntStatus::Done;
                        st0.insert(St0::SEEK_END);
                        //if no floppy inserted: st0.insert(St0::FAIL | St0::SEEK_END | St0::NOT_READY); //St0::SEEK_END);
                    } else {
                        st0.insert(St0::UNKNOWN);
                    }
                    self.reply.set(&[st0.bits(), self.cylinder]);
                    //log::debug!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            0x0f => {
                if len == 3 {
                    //log::debug!("Seek {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let _head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    if drive == 0 {
                        self.int_seek_completed = IntStatus::Running;
                        self.cylinder = self.cmd[2];
                        self.reply.set(&[]);
                    }
                    self.cmd.clear();
                }
            }
            c if c & 0b0001_1111 == 0b0001_0000 => {
                if len == 1 {
                    //log::debug!("Version {:02x?}", self.cmd);
                    self.reply.set(&[0x80]); // PD765A
                    //log::debug!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            // from here on, masks
            c if c & 0b11111 == 0b00110 || c & 0b11111 == 0b01100 => {
                if len == 9 {
                    let deleted = c & 0b11111 == 0b01100;
                    let skip = c & 0x20 != 0;
                    let multitrack = c & 0x80 != 0;
                    log::debug!(
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

                    if drive == 0
                        && let Some(disk) = self.disk.as_mut()
                    {
                        let id = SectorId { c, h, r, n };
                        if let Some(sector) = disk
                            .get_track(head, self.cylinder)
                            .and_then(|track| track.get_sector(&id))
                        {
                            let mut expected_len = sector.id.len();
                            self.data.set(&sector.data);
                            //log::debug!("Reading {} {} = {}", self.cylinder, r, sector.data.len());

                            let st0 = St0::from_c1(c1) | St0::FAIL;
                            let mut st1 = sector.st1;
                            let mut st2 = sector.st2;
                            if deleted {
                                st2.toggle(St2::CONTROL_MARK);
                            }

                            if eot > r {
                                // TODO eot < r???

                                //log::debug!("Multi sector read!!!");
                                //let mut cur = id;
                                let track = disk.get_track(head, self.cylinder).unwrap();
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
                                    self.data.get_mut().extend(&next.data);
                                    //log::debug!("   extend {}", next.id.r);
                                    //if next.id.r == eot {
                                    //    break;
                                    //}
                                    //cur = next.id.clone();
                                }
                            }
                            if self.data.get_ref().len() == expected_len {
                                st1.insert(St1::END_OF_CYLINDER);
                            } else {
                                st1.insert(St1::OVERRUN);
                            }

                            self.reply
                                .set(&[st0.bits(), st1.bits(), st2.bits(), c, h, eot, n]);
                        } else {
                            let st0 = St0::from_c1(c1) | St0::FAIL;
                            let st1 = St1::MISSING_AM;
                            self.reply
                                .set(&[st0.bits(), st1.bits(), 0, 0xff, 0xff, 0xff, 0xff]);
                        }
                    } else {
                        let st0 = St0::from_c1(c1) | St0::FAIL | St0::UNKNOWN | St0::NOT_READY;
                        self.reply.set(&[st0.bits(), 0, 0, c, h, r, n]);
                    }

                    //log::debug!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            c if c & 0b111111 == 0b000101 => {
                if len == 9 {
                    log::debug!("Write data {:02x?}", self.cmd);
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
                    if drive == 0
                        && let Some(disk) = self.disk.as_mut()
                    {
                        let sid = SectorId { c, h, r, n };
                        if let Some(_sector) = disk
                            .get_track_mut(head, self.cylinder)
                            .and_then(|track| track.get_sector_mut(&sid))
                        {
                            let len = sid.len();
                            self.data_in = Some((c1, sid, len, Vec::new()));
                        } else {
                            let st0 = St0::from_c1(c1) | St0::FAIL;
                            let st1 = St1::MISSING_AM;
                            self.reply
                                .set(&[st0.bits(), st1.bits(), 0, 0xff, 0xff, 0xff, 0xff]);
                        }
                    } else {
                        let st0 = St0::from_c1(c1) | St0::FAIL | St0::UNKNOWN | St0::NOT_READY;
                        self.reply.set(&[st0.bits(), 0, 0, 0xff, 0xff, 0xff, 0xff]);
                    }
                    self.cmd.clear();
                }
            }
            c if c & 0b111111 == 0b001001 => {
                if len == 9 {
                    //log::debug!("Write deleted data *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b1001_1111 == 0b0000_0010 => {
                if len == 9 {
                    // or is it read track?
                    //log::debug!("Read diagnostic *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b1011_1111 == 0b0000_1010 => {
                if len == 2 {
                    //log::debug!("Read ID {:02x?} at cyl {}", self.cmd, self.cylinder);
                    let c1 = self.cmd[1];
                    let head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    if drive == 0
                        && let Some(disk) = self.disk.as_mut()
                    {
                        if let Some(sector) = disk
                            .get_track(head, self.cylinder)
                            .and_then(|track| track.get_sector_by_idx(self.read_id_idx as usize))
                        {
                            let id = &sector.id;
                            let st0 = St0::from_c1(c1);
                            self.reply.set(&[st0.bits(), 0, 0, id.c, id.h, id.r, id.n]);
                        } else {
                            let st0 = St0::from_c1(c1) | St0::FAIL;
                            let st1 = St1::MISSING_AM;
                            self.reply
                                .set(&[st0.bits(), st1.bits(), 0, 0xff, 0xff, 0xff, 0xff]);
                        }
                        self.read_id_idx = self.read_id_idx.wrapping_add(1);
                    }

                    if self.reply.is_empty() {
                        let st0 = St0::from_c1(c1) | St0::UNKNOWN | St0::FAIL | St0::NOT_READY;
                        self.reply.set(&[st0.bits(), 0, 0, 0, 0, 0, 2]);
                    }
                    //log::debug!("<<< {:02x?}", self.reply);
                    self.cmd.clear();
                }
            }
            c if c & 0b1011_1111 == 0b0000_1101 => {
                if len == 6 {
                    // AKA format track
                    //log::debug!("Write ID {:02x?}", self.cmd);
                    let c1 = self.cmd[1];
                    let head = (c1 & 0b0100 != 0) as u8;
                    let drive = c1 & 0b0011;
                    let n = self.cmd[2];
                    let sectors = self.cmd[3];
                    let gpl = self.cmd[4];
                    let filler = self.cmd[5];

                    if drive == 0
                        && let Some(disk) = self.disk.as_mut()
                    {
                        disk.set_track(
                            head,
                            self.cylinder,
                            Track::new_formatted(self.cylinder, head, n, sectors, gpl, filler),
                        );
                        let st0 = St0::from_c1(c1);
                        self.reply.set(&[st0.bits(), 0, 0, 0, 0, 0, n]);
                    } else {
                        let st0 = St0::from_c1(c1) | St0::UNKNOWN | St0::FAIL | St0::NOT_READY;
                        self.reply
                            .set(&[st0.bits(), 0, 0, self.cylinder, head, 0, n]);
                    }
                    //log::debug!("<<< {:02x?}", self.reply);

                    self.cmd.clear();
                }
            }
            c if c & 0b11111 == 0b10001 => {
                if len == 9 {
                    //log::debug!("Scan equal *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b11111 == 0b11001 => {
                if len == 9 {
                    //log::debug!("Scan low or equal *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            c if c & 0b11111 == 0b11101 => {
                if len == 9 {
                    //log::debug!("Scan high or equal *TODO* {:02x?}", self.cmd);
                    self.cmd.clear();
                }
            }
            _c => {
                if len == 1 {
                    //log::debug!("Invalid {_c:02x}");
                    self.cmd.clear();
                }
            }
        };
    }
}
