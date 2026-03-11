use anyhow::bail;
use std::io::{Read, Seek, SeekFrom};

use crate::{
    floppy::{SectorId, St1, St2},
    ReadExt,
};

#[derive(Debug)]
pub struct Disk {
    tracks: Vec<Track>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Track {
    cylinder: u8,
    side: u8,
    sector_size: u8,
    gap3: u8,
    filler: u8,
    sectors: Vec<Sector>,
}

impl Track {
    pub fn new_formatted(c: u8, h: u8, n: u8, nsectors: u8, gap3: u8, filler: u8) -> Track {
        let mut sectors = Vec::new();

        for sector in 0..nsectors {
            let id = SectorId {
                c,
                h,
                r: sector + 1,
                n,
            };
            let data = vec![filler; id.len()];
            let sector = Sector {
                id,
                st1: St1::empty(),
                st2: St2::empty(),
                data,
            };
            sectors.push(sector);
        }

        Track {
            sectors,
            cylinder: c,
            side: h,
            sector_size: n,
            gap3,
            filler,
        }
    }

    pub fn get_sector(&self, id: &SectorId) -> Option<&Sector> {
        dbg!(id);
        self.sectors.iter().find(|s| s.id == *id)
    }

    pub fn get_sector_mut(&mut self, id: &SectorId) -> Option<&mut Sector> {
        self.sectors.iter_mut().find(|s| s.id == *id)
    }

    pub fn get_sector_by_idx(&self, idx: usize) -> Option<&Sector> {
        if self.sectors.is_empty() {
            return None;
        }
        Some(&self.sectors[idx % self.sectors.len()])
    }
}

pub struct Sector {
    pub id: SectorId,
    pub st1: St1,
    pub st2: St2,
    pub data: Vec<u8>,
}

impl std::fmt::Debug for Sector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sector")
            .field("id", &self.id)
            .field("st1", &self.st1)
            .field("st2", &self.st2)
            .field("data", &self.data.len())
            .finish()
    }
}

impl Disk {
    pub fn new_formatted() -> Disk {
        let tracks = (0..40)
            .map(|c| Track::new_formatted(c, 0, 2, 9, 0x2f, 0xe5))
            .collect();
        Disk { tracks }
    }

    pub fn new<R: Read + Seek>(mut rdr: R) -> anyhow::Result<Disk> {
        enum FileVer {
            Normal,
            Extended,
        }

        let signature = rdr.read_string(12)?;
        let file_ver = if signature == "EXTENDED CPC" {
            FileVer::Extended
        } else if signature.starts_with("MV - CPC") {
            FileVer::Normal
        } else {
            bail!("Invalid EXTENDED CPC");
        };

        // Header strings are somewhat inconsistent
        rdr.seek(SeekFrom::Start(0x30))?;
        let tracks = rdr.read_u8()?;
        let sides = rdr.read_u8()?;
        dbg!(tracks, sides);

        let num_tracks = tracks as usize * sides as usize;
        let mut track_sizes = Vec::with_capacity(num_tracks);
        match file_ver {
            FileVer::Normal => {
                let track_size = rdr.read_u16()?;
                track_sizes.resize_with(num_tracks, || Some(track_size));
            }
            FileVer::Extended => {
                rdr.read_u16()?;
                for _ in 0..num_tracks {
                    let track_size = (rdr.read_u8()? as u16) << 8;
                    if track_size == 0 {
                        track_sizes.push(None);
                    } else {
                        track_sizes.push(Some(track_size));
                    }
                }
            }
        };

        let mut tracks = Vec::with_capacity(track_sizes.len());
        let mut pos = 0x100;

        for track_size in track_sizes {
            let Some(track_size) = track_size else {
                continue;
            };

            rdr.seek(SeekFrom::Start(pos))?;

            let sig = rdr.read_string(12)?;

            if sig != "Track-Info\r\n" {
                bail!("Invalid Track-Info signature");
            }

            rdr.read_u32()?;
            let cylinder = rdr.read_u8()?;
            let side = rdr.read_u8()?;
            rdr.read_u16()?;
            let sector_size = rdr.read_u8()?;
            let num_sectors = rdr.read_u8()?;
            let gap3 = rdr.read_u8()?;
            let filler = rdr.read_u8()?;

            let mut sectors = Vec::with_capacity(num_sectors as usize);
            for _ in 0..num_sectors {
                let c = rdr.read_u8()?;
                let h = rdr.read_u8()?;
                let r = rdr.read_u8()?;
                let n = rdr.read_u8()?;
                let id = SectorId { c, h, r, n };
                let st1 = St1::from_bits_retain(rdr.read_u8()?);
                let st2 = St2::from_bits_retain(rdr.read_u8()?);
                let file_len = rdr.read_u16()?;

                let real_len = match file_ver {
                    FileVer::Normal => id.len(),
                    FileVer::Extended => file_len as usize,
                };

                let data = vec![0; real_len];

                let sector = Sector { id, st1, st2, data };
                sectors.push(sector);
            }

            rdr.seek(SeekFrom::Start(pos + 0x100))?;

            for sector in &mut sectors {
                rdr.read_exact(&mut sector.data)?;
            }

            let track = Track {
                cylinder,
                side,
                sector_size,
                gap3,
                filler,
                sectors,
            };
            tracks.push(track);

            pos += track_size as u64;
        }
        Ok(Disk { tracks })
    }

    pub fn get_track(&self, side: u8, cylinder: u8) -> Option<&Track> {
        self.tracks
            .iter()
            .find(|track| track.cylinder == cylinder && track.side == side)
    }
    pub fn get_track_mut(&mut self, side: u8, cylinder: u8) -> Option<&mut Track> {
        self.tracks
            .iter_mut()
            .find(|track| track.cylinder == cylinder && track.side == side)
    }

    pub fn set_track(&mut self, side: u8, cylinder: u8, new_track: Track) {
        match self
            .tracks
            .iter_mut()
            .find(|track| track.cylinder == cylinder && track.side == side)
        {
            Some(old) => *old = new_track,
            None => {
                self.tracks.push(new_track);
                self.tracks.sort_by_key(|t| (t.cylinder, t.side));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test1() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("../files/disks/test.dsk");

        let f = std::fs::File::open(&d).unwrap();
        let mut f = std::io::BufReader::new(f);
        let d = Disk::new(&mut f).unwrap();
        dbg!(d);
    }
}
