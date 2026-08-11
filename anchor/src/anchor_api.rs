#![allow(unused)]
pub const CREATE_DELETE_SWITCH: usize = 1;
pub const CREATE_SWITCH: usize = 2;
pub const SWITCH: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum ErrorCode {
    EntryOutsideRegion = 0,
    UnmatchedRegionSize = 1,
    OverlappingRegion = 2,
    RegionDoesNotExist = 3,
}
