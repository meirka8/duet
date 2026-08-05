use duet_types::{EntryId, FileType};

/// Memory budget target per entry in bytes (excluding string arena data).
pub const PER_ENTRY_BYTE_BUDGET: usize = 96;

/// Struct-of-Arrays (SoA) layout for high-density entry storage with interned name arena.
#[derive(Debug, Clone, Default)]
pub struct EntryStore {
    name_arena: Vec<u8>,
    ids: Vec<EntryId>,
    name_offsets: Vec<u32>,
    name_lens: Vec<u16>,
    file_types: Vec<FileType>,
    sizes: Vec<u64>,
    modes: Vec<u32>,
    uids: Vec<u32>,
    gids: Vec<u32>,
    mtimes: Vec<i64>,
    atimes: Vec<i64>,
    ctimes: Vec<i64>,
    devs: Vec<u64>,
    inos: Vec<u64>,
    nlinks: Vec<u32>,
    flags: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct EntryRecord<'a> {
    pub id: EntryId,
    pub name: &'a str,
    pub file_type: FileType,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: i64,
    pub atime: i64,
    pub ctime: i64,
    pub dev: u64,
    pub ino: u64,
    pub nlink: u32,
    pub flags: u32,
}

impl EntryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            name_arena: Vec::with_capacity(capacity * 16),
            ids: Vec::with_capacity(capacity),
            name_offsets: Vec::with_capacity(capacity),
            name_lens: Vec::with_capacity(capacity),
            file_types: Vec::with_capacity(capacity),
            sizes: Vec::with_capacity(capacity),
            modes: Vec::with_capacity(capacity),
            uids: Vec::with_capacity(capacity),
            gids: Vec::with_capacity(capacity),
            mtimes: Vec::with_capacity(capacity),
            atimes: Vec::with_capacity(capacity),
            ctimes: Vec::with_capacity(capacity),
            devs: Vec::with_capacity(capacity),
            inos: Vec::with_capacity(capacity),
            nlinks: Vec::with_capacity(capacity),
            flags: Vec::with_capacity(capacity),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push(
        &mut self,
        id: EntryId,
        name: &str,
        file_type: FileType,
        size: u64,
        mode: u32,
        uid: u32,
        gid: u32,
        mtime: i64,
        atime: i64,
        ctime: i64,
        dev: u64,
        ino: u64,
        nlink: u32,
        flags: u32,
    ) -> usize {
        let name_bytes = name.as_bytes();
        let offset = self.name_arena.len() as u32;
        let len = name_bytes.len() as u16;
        self.name_arena.extend_from_slice(name_bytes);

        let index = self.ids.len();
        self.ids.push(id);
        self.name_offsets.push(offset);
        self.name_lens.push(len);
        self.file_types.push(file_type);
        self.sizes.push(size);
        self.modes.push(mode);
        self.uids.push(uid);
        self.gids.push(gid);
        self.mtimes.push(mtime);
        self.atimes.push(atime);
        self.ctimes.push(ctime);
        self.devs.push(dev);
        self.inos.push(ino);
        self.nlinks.push(nlink);
        self.flags.push(flags);

        index
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn clear(&mut self) {
        self.name_arena.clear();
        self.ids.clear();
        self.name_offsets.clear();
        self.name_lens.clear();
        self.file_types.clear();
        self.sizes.clear();
        self.modes.clear();
        self.uids.clear();
        self.gids.clear();
        self.mtimes.clear();
        self.atimes.clear();
        self.ctimes.clear();
        self.devs.clear();
        self.inos.clear();
        self.nlinks.clear();
        self.flags.clear();
    }

    pub fn get_name(&self, index: usize) -> &str {
        let offset = self.name_offsets[index] as usize;
        let len = self.name_lens[index] as usize;
        std::str::from_utf8(&self.name_arena[offset..offset + len])
            .unwrap_or("<invalid utf8>")
    }

    pub fn get(&self, index: usize) -> EntryRecord<'_> {
        EntryRecord {
            id: self.ids[index],
            name: self.get_name(index),
            file_type: self.file_types[index],
            size: self.sizes[index],
            mode: self.modes[index],
            uid: self.uids[index],
            gid: self.gids[index],
            mtime: self.mtimes[index],
            atime: self.atimes[index],
            ctime: self.ctimes[index],
            dev: self.devs[index],
            ino: self.inos[index],
            nlink: self.nlinks[index],
            flags: self.flags[index],
        }
    }

    pub fn id(&self, index: usize) -> EntryId {
        self.ids[index]
    }

    pub fn file_type(&self, index: usize) -> FileType {
        self.file_types[index]
    }

    pub fn size(&self, index: usize) -> u64 {
        self.sizes[index]
    }

    pub fn mtime(&self, index: usize) -> i64 {
        self.mtimes[index]
    }

    pub fn mode(&self, index: usize) -> u32 {
        self.modes[index]
    }

    pub fn update_entry(
        &mut self,
        index: usize,
        size: u64,
        mode: u32,
        mtime: i64,
        atime: i64,
        ctime: i64,
    ) {
        self.sizes[index] = size;
        self.modes[index] = mode;
        self.mtimes[index] = mtime;
        self.atimes[index] = atime;
        self.ctimes[index] = ctime;
    }

    /// Calculate the struct-of-arrays memory overhead per entry in bytes (excluding string arena).
    pub fn per_entry_bytes_soa() -> usize {
        std::mem::size_of::<EntryId>()       // 8
            + std::mem::size_of::<u32>()     // 4 (name_offset)
            + std::mem::size_of::<u16>()     // 2 (name_len)
            + std::mem::size_of::<FileType>()// 1
            + std::mem::size_of::<u64>()     // 8 (size)
            + std::mem::size_of::<u32>()     // 4 (mode)
            + std::mem::size_of::<u32>()     // 4 (uid)
            + std::mem::size_of::<u32>()     // 4 (gid)
            + std::mem::size_of::<i64>()     // 8 (mtime)
            + std::mem::size_of::<i64>()     // 8 (atime)
            + std::mem::size_of::<i64>()     // 8 (ctime)
            + std::mem::size_of::<u64>()     // 8 (dev)
            + std::mem::size_of::<u64>()     // 8 (ino)
            + std::mem::size_of::<u32>()     // 4 (nlink)
            + std::mem::size_of::<u32>()     // 4 (flags)
    }

    /// Measure actual memory allocated per entry across all SoA vectors.
    pub fn memory_usage_bytes(&self) -> usize {
        self.name_arena.capacity()
            + self.ids.capacity() * std::mem::size_of::<EntryId>()
            + self.name_offsets.capacity() * std::mem::size_of::<u32>()
            + self.name_lens.capacity() * std::mem::size_of::<u16>()
            + self.file_types.capacity() * std::mem::size_of::<FileType>()
            + self.sizes.capacity() * std::mem::size_of::<u64>()
            + self.modes.capacity() * std::mem::size_of::<u32>()
            + self.uids.capacity() * std::mem::size_of::<u32>()
            + self.gids.capacity() * std::mem::size_of::<u32>()
            + self.mtimes.capacity() * std::mem::size_of::<i64>()
            + self.atimes.capacity() * std::mem::size_of::<i64>()
            + self.ctimes.capacity() * std::mem::size_of::<i64>()
            + self.devs.capacity() * std::mem::size_of::<u64>()
            + self.inos.capacity() * std::mem::size_of::<u64>()
            + self.nlinks.capacity() * std::mem::size_of::<u32>()
            + self.flags.capacity() * std::mem::size_of::<u32>()
    }
}
