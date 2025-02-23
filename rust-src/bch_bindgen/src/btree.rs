use crate::SPOS_MAX;
use crate::c;
use crate::bkey::BkeySC;
use crate::fs::Fs;
use crate::errcode::{bch_errcode, errptr_to_result_c};
use crate::printbuf_to_formatter;
use std::fmt;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;
use bitflags::bitflags;

pub struct BtreeTrans<'f> {
    raw:    *mut c::btree_trans,
    fs:     PhantomData<&'f Fs>
}

impl<'f> BtreeTrans<'f> {
    pub fn new(fs: &'f Fs) -> BtreeTrans {
        unsafe {
            BtreeTrans { raw: &mut *c::__bch2_trans_get(fs.raw, 0), fs: PhantomData }
        }
    }
}

impl<'f> Drop for BtreeTrans<'f> {
    fn drop(&mut self) {
        unsafe { c::bch2_trans_put(&mut *self.raw) }
    }             
}

bitflags! {
    pub struct BtreeIterFlags: u16 {
        const SLOTS = c::BTREE_ITER_SLOTS as u16;
        const ALL_LEVELS = c::BTREE_ITER_ALL_LEVELS as u16;
        const INTENT = c::BTREE_ITER_INTENT	 as u16;
        const PREFETCH = c::BTREE_ITER_PREFETCH as u16;
        const IS_EXTENTS = c::BTREE_ITER_IS_EXTENTS as u16;
        const NOT_EXTENTS = c::BTREE_ITER_NOT_EXTENTS as u16;
        const CACHED = c::BTREE_ITER_CACHED	as u16;
        const KEY_CACHED = c::BTREE_ITER_WITH_KEY_CACHE as u16;
        const WITH_UPDATES = c::BTREE_ITER_WITH_UPDATES as u16;
        const WITH_JOURNAL = c::BTREE_ITER_WITH_JOURNAL as u16;
        const __ALL_SNAPSHOTS = c::__BTREE_ITER_ALL_SNAPSHOTS as u16;
        const ALL_SNAPSHOTS = c::BTREE_ITER_ALL_SNAPSHOTS as u16;
        const FILTER_SNAPSHOTS = c::BTREE_ITER_FILTER_SNAPSHOTS as u16;
        const NOPRESERVE = c::BTREE_ITER_NOPRESERVE as u16;
        const CACHED_NOFILL = c::BTREE_ITER_CACHED_NOFILL as u16;
        const KEY_CACHE_FILL = c::BTREE_ITER_KEY_CACHE_FILL as u16;
    }
}

pub struct BtreeIter<'t> {
    raw:    c::btree_iter,
    trans:  PhantomData<&'t BtreeTrans<'t>>,
}

impl<'t> BtreeIter<'t> {
    pub fn new(trans: &'t BtreeTrans<'t>, btree: c::btree_id, pos: c::bpos, flags: BtreeIterFlags) -> BtreeIter<'t> {
        unsafe {
            let mut iter: MaybeUninit<c::btree_iter> = MaybeUninit::uninit();

            c::bch2_trans_iter_init_outlined(
                trans.raw,
                iter.as_mut_ptr(),
                btree,
                pos,
                flags.bits as u32);

            BtreeIter { raw: iter.assume_init(), trans: PhantomData }
        }
    }

    pub fn peek_upto<'i>(&'i mut self, end: c::bpos) -> Result<Option<BkeySC>, bch_errcode> {
        unsafe {
            let k = c::bch2_btree_iter_peek_upto(&mut self.raw, end);
            errptr_to_result_c(k.k)
                .map(|_| if !k.k.is_null() { Some(BkeySC { k: &*k.k, v: &*k.v, iter: PhantomData }) } else { None } )
        }
    }

    pub fn peek(&mut self) -> Result<Option<BkeySC>, bch_errcode> {
        self.peek_upto(SPOS_MAX)
    }

    pub fn peek_and_restart(&mut self) -> Result<Option<BkeySC>, bch_errcode> {
        unsafe {
            let k = c::bch2_btree_iter_peek_and_restart_outlined(&mut self.raw);

            errptr_to_result_c(k.k)
                .map(|_| if !k.k.is_null() { Some(BkeySC{ k: &*k.k, v: &*k.v, iter: PhantomData }) } else { None } )
        }
    }

    pub fn advance(&mut self) {
        unsafe {
            c::bch2_btree_iter_advance(&mut self.raw);
        }
    }
}

impl<'t> Drop for BtreeIter<'t> {
    fn drop(&mut self) {
        unsafe { c::bch2_trans_iter_exit(self.raw.trans, &mut self.raw) }
    }             
}

pub struct BtreeNodeIter<'t> {
    raw:    c::btree_iter,
    trans:  PhantomData<&'t BtreeTrans<'t>>,
}

impl<'t> BtreeNodeIter<'t> {
    pub fn new(trans: &'t BtreeTrans<'t>,
        btree:      c::btree_id,
        pos:        c::bpos,
        locks_want: u32,
        depth:      u32,
        flags: BtreeIterFlags) -> BtreeNodeIter {
        unsafe {
            let mut iter: MaybeUninit<c::btree_iter> = MaybeUninit::uninit();
            c::bch2_trans_node_iter_init(
                trans.raw,
                iter.as_mut_ptr(),
                btree,
                pos,
                locks_want,
                depth,
                flags.bits as u32);

            BtreeNodeIter { raw: iter.assume_init(), trans: PhantomData }
        }
    }

    pub fn peek<'i>(&'i mut self) -> Result<Option<&'i c::btree>, bch_errcode> {
        unsafe {
            let b = c::bch2_btree_iter_peek_node(&mut self.raw);
            errptr_to_result_c(b).map(|b| if !b.is_null() { Some(&*b) } else { None })
        }
    }

    pub fn peek_and_restart<'i>(&'i mut self) -> Result<Option<&'i c::btree>, bch_errcode> {
        unsafe {
            let b = c::bch2_btree_iter_peek_node_and_restart(&mut self.raw);
            errptr_to_result_c(b).map(|b| if !b.is_null() { Some(&*b) } else { None })
        }
    }

    pub fn advance<'i>(&'i mut self) {
        unsafe {
            c::bch2_btree_iter_next_node(&mut self.raw);
        }
    }

    pub fn next<'i>(&'i mut self) -> Result<Option<&'i c::btree>, bch_errcode> {
        unsafe {
            let b = c::bch2_btree_iter_next_node(&mut self.raw);
            errptr_to_result_c(b).map(|b| if !b.is_null() { Some(&*b) } else { None })
        }
    }
}

impl<'t> Drop for BtreeNodeIter<'t> {
    fn drop(&mut self) {
        unsafe { c::bch2_trans_iter_exit(self.raw.trans, &mut self.raw) }
    }             
}

impl<'b, 'f> c::btree {
    pub fn to_text(&'b self, fs: &'f Fs) -> BtreeNodeToText<'b, 'f> {
        BtreeNodeToText { b: &self, fs }
    }

    pub fn ondisk_to_text(&'b self, fs: &'f Fs) -> BtreeNodeOndiskToText<'b, 'f> {
        BtreeNodeOndiskToText { b: &self, fs }
    }
}

pub struct BtreeNodeToText<'b, 'f> {
    b:      &'b c::btree,
    fs:     &'f Fs,
}

impl<'b, 'f> fmt::Display for BtreeNodeToText<'b, 'f> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        printbuf_to_formatter(f, |buf| unsafe { c::bch2_btree_node_to_text(buf, self.fs.raw, self.b) })
    }
}

pub struct BtreeNodeOndiskToText<'b, 'f> {
    b:      &'b c::btree,
    fs:     &'f Fs,
}

impl<'b, 'f> fmt::Display for BtreeNodeOndiskToText<'b, 'f> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        printbuf_to_formatter(f, |buf| unsafe { c::bch2_btree_node_ondisk_to_text(buf, self.fs.raw, self.b) })
    }
}
