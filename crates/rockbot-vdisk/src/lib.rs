use aes::cipher::KeyInit;
use aes::Aes256;
use anyhow::{bail, Context, Result};
use redb::StorageBackend;
use ring::hkdf;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use xts_mode::{get_tweak_default, Xts128};

const MAGIC: &[u8; 8] = b"RBVDISK1";
const HEADER_BYTES: u64 = 1024 * 1024;
const ALIGNMENT: u64 = 4096;
const XTS_KEY_INFO_PREFIX: &[u8] = b"rockbot-vdisk/aes-256-xts/v1:";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiskManifest {
    version: u32,
    volumes: BTreeMap<String, VolumeRecord>,
}

impl Default for DiskManifest {
    fn default() -> Self {
        Self {
            version: 1,
            volumes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VolumeRecord {
    offset: u64,
    capacity: u64,
    len: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeInfo {
    pub offset: u64,
    pub capacity: u64,
    pub len: u64,
}

#[derive(Debug)]
struct VolumeState {
    file: File,
    manifest: DiskManifest,
}

/// A named virtual volume inside `rockbot.data`.
pub struct VolumeBackend {
    inner: Mutex<VolumeState>,
    key: Option<[u8; 64]>,
    volume_name: String,
    base_offset: u64,
    capacity: u64,
    logical_len: AtomicU64,
}

impl fmt::Debug for VolumeBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VolumeBackend")
            .field("volume_name", &self.volume_name)
            .field("base_offset", &self.base_offset)
            .field("capacity", &self.capacity)
            .field("logical_len", &self.logical_len.load(Ordering::Relaxed))
            .field("encrypted", &self.key.is_some())
            .finish()
    }
}

impl VolumeBackend {
    pub fn open(
        disk_path: &Path,
        volume_name: &str,
        capacity: u64,
        key: Option<[u8; 32]>,
    ) -> Result<Self> {
        if volume_name.is_empty() {
            bail!("volume name must not be empty");
        }
        if capacity == 0 {
            bail!("volume capacity must be greater than zero");
        }

        if let Some(parent) = disk_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create parent directory for {}",
                    disk_path.display()
                )
            })?;
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(disk_path)
            .with_context(|| format!("failed to open virtual disk {}", disk_path.display()))?;

        let mut manifest = Self::load_or_initialize_manifest(&mut file)?;
        let volume = if let Some(existing) = manifest.volumes.get(volume_name).cloned() {
            existing
        } else {
            let next_offset = manifest
                .volumes
                .values()
                .map(|v| v.offset + v.capacity)
                .max()
                .unwrap_or(HEADER_BYTES);
            let offset = align_up(next_offset, ALIGNMENT);
            let capacity = align_up(capacity, ALIGNMENT);
            let volume = VolumeRecord {
                offset,
                capacity,
                len: 0,
            };
            manifest
                .volumes
                .insert(volume_name.to_string(), volume.clone());
            Self::persist_manifest(&mut file, &manifest)?;
            let required_len = offset + capacity;
            if file.metadata()?.len() < required_len {
                file.set_len(required_len)?;
            }
            volume
        };

        let required_len = volume.offset + volume.capacity;
        if file.metadata()?.len() < required_len {
            file.set_len(required_len)?;
        }

        Ok(Self {
            inner: Mutex::new(VolumeState { file, manifest }),
            key: key
                .map(|root_key| Self::derive_xts_key(&root_key, volume_name.as_bytes()))
                .transpose()?,
            volume_name: volume_name.to_string(),
            base_offset: volume.offset,
            capacity: volume.capacity,
            logical_len: AtomicU64::new(volume.len),
        })
    }

    pub fn encrypted(&self) -> bool {
        self.key.is_some()
    }

    pub fn logical_len(&self) -> u64 {
        self.logical_len.load(Ordering::Relaxed)
    }

    pub fn write_all(&self, data: &[u8]) -> Result<()> {
        <Self as StorageBackend>::write(self, 0, data)?;
        <Self as StorageBackend>::set_len(self, data.len() as u64)?;
        Ok(())
    }

    pub fn read_all(&self) -> Result<Vec<u8>> {
        let len = self.logical_len() as usize;
        Ok(<Self as StorageBackend>::read(self, 0, len)?)
    }

    fn load_or_initialize_manifest(file: &mut File) -> Result<DiskManifest> {
        let file_len = file.metadata()?.len();
        if file_len == 0 {
            file.set_len(HEADER_BYTES)?;
            let manifest = DiskManifest::default();
            Self::persist_manifest(file, &manifest)?;
            return Ok(manifest);
        }

        let mut magic = [0u8; 8];
        file.seek(SeekFrom::Start(0))?;
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            bail!("{} is not a rockbot virtual disk", file_descriptor(file));
        }

        let mut header_len_bytes = [0u8; 8];
        file.read_exact(&mut header_len_bytes)?;
        let header_len = u64::from_le_bytes(header_len_bytes);
        if header_len == 0 || header_len > (HEADER_BYTES - 16) {
            bail!("virtual disk header is corrupt");
        }
        let mut json = vec![0u8; header_len as usize];
        file.read_exact(&mut json)?;
        serde_json::from_slice::<DiskManifest>(&json).context("failed to parse virtual disk header")
    }

    fn persist_manifest(file: &mut File, manifest: &DiskManifest) -> Result<()> {
        let encoded = serde_json::to_vec(manifest)?;
        let max_len = (HEADER_BYTES - 16) as usize;
        if encoded.len() > max_len {
            bail!("virtual disk manifest exceeds reserved header size");
        }

        file.seek(SeekFrom::Start(0))?;
        file.write_all(MAGIC)?;
        file.write_all(&(encoded.len() as u64).to_le_bytes())?;
        file.write_all(&encoded)?;
        let remaining = max_len - encoded.len();
        if remaining > 0 {
            let zeros = vec![0u8; remaining];
            file.write_all(&zeros)?;
        }
        file.sync_data()?;
        Ok(())
    }

    fn derive_xts_key(root_key: &[u8; 32], volume_name: &[u8]) -> io::Result<[u8; 64]> {
        struct XtsKeyLen;
        impl hkdf::KeyType for XtsKeyLen {
            fn len(&self) -> usize {
                64
            }
        }

        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"rockbot-vdisk-root-key");
        let prk = salt.extract(root_key);
        let mut info = Vec::with_capacity(XTS_KEY_INFO_PREFIX.len() + volume_name.len());
        info.extend_from_slice(XTS_KEY_INFO_PREFIX);
        info.extend_from_slice(volume_name);
        let info_refs = [info.as_slice()];
        let okm = prk
            .expand(&info_refs, XtsKeyLen)
            .map_err(|_| io::Error::other("invalid AES-XTS key derivation context"))?;
        let mut derived = [0u8; 64];
        okm.fill(&mut derived)
            .map_err(|_| io::Error::other("failed to derive AES-XTS key"))?;
        Ok(derived)
    }

    fn xts_cipher(&self) -> Option<Xts128<Aes256>> {
        let key = self.key.as_ref()?;
        let cipher_1 = Aes256::new_from_slice(&key[..32]).expect("valid AES-256 key");
        let cipher_2 = Aes256::new_from_slice(&key[32..]).expect("valid AES-256 key");
        Some(Xts128::new(cipher_1, cipher_2))
    }

    fn apply_xts(&self, absolute_offset: u64, data: &mut [u8], encrypt: bool) {
        let Some(xts) = self.xts_cipher() else {
            return;
        };
        if data.is_empty() {
            return;
        }
        let first_sector = u128::from(absolute_offset / ALIGNMENT);
        if encrypt {
            xts.encrypt_area(data, ALIGNMENT as usize, first_sector, get_tweak_default);
        } else {
            xts.decrypt_area(data, ALIGNMENT as usize, first_sector, get_tweak_default);
        }
    }

    fn checked_end_offset(&self, offset: u64, len: usize) -> io::Result<u64> {
        let len_u64 = u64::try_from(len).map_err(|_| io::Error::other("length overflow"))?;
        let end = offset
            .checked_add(len_u64)
            .ok_or_else(|| io::Error::other("offset overflow"))?;
        if end > self.capacity {
            return Err(io::Error::other(format!(
                "virtual volume '{}' exceeded capacity ({end} > {})",
                self.volume_name, self.capacity
            )));
        }
        Ok(end)
    }

    fn absolute_offset(&self, relative_offset: u64) -> io::Result<u64> {
        self.base_offset
            .checked_add(relative_offset)
            .ok_or_else(|| io::Error::other("absolute offset overflow"))
    }

    fn set_len_locked(&self, guard: &mut VolumeState, len: u64) -> io::Result<()> {
        let record = guard
            .manifest
            .volumes
            .get_mut(&self.volume_name)
            .ok_or_else(|| io::Error::other("virtual volume missing from manifest"))?;
        record.len = len;
        let manifest = guard.manifest.clone();
        Self::persist_manifest(&mut guard.file, &manifest)
            .map_err(|err| io::Error::other(err.to_string()))?;
        self.logical_len.store(len, Ordering::Relaxed);
        Ok(())
    }
}

impl StorageBackend for VolumeBackend {
    fn len(&self) -> Result<u64, io::Error> {
        Ok(self.logical_len())
    }

    fn read(&self, offset: u64, len: usize) -> Result<Vec<u8>, io::Error> {
        let logical_len = self.logical_len();
        let end = self.checked_end_offset(offset, len)?;
        if end > logical_len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "read beyond logical end of volume '{}' ({end} > {logical_len})",
                    self.volume_name
                ),
            ));
        }

        let absolute = self.absolute_offset(offset)?;
        let sector_start = align_down(absolute, ALIGNMENT);
        let sector_end =
            align_up(absolute + len as u64, ALIGNMENT).min(self.base_offset + logical_len);
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("virtual disk mutex poisoned"))?;
        guard.file.seek(SeekFrom::Start(sector_start))?;
        let mut buf = vec![0u8; (sector_end - sector_start) as usize];
        guard.file.read_exact(&mut buf)?;
        drop(guard);
        self.apply_xts(sector_start, &mut buf, false);
        let relative = (absolute - sector_start) as usize;
        Ok(buf[relative..relative + len].to_vec())
    }

    fn set_len(&self, len: u64) -> Result<(), io::Error> {
        if len > self.capacity {
            return Err(io::Error::other(format!(
                "virtual volume '{}' exceeded capacity ({} > {})",
                self.volume_name, len, self.capacity
            )));
        }

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("virtual disk mutex poisoned"))?;
        self.set_len_locked(&mut guard, len)
    }

    fn sync_data(&self, _eventual: bool) -> Result<(), io::Error> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("virtual disk mutex poisoned"))?;
        guard.file.sync_data()
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<(), io::Error> {
        let end = self.checked_end_offset(offset, data.len())?;
        let absolute = self.absolute_offset(offset)?;
        let sector_start = align_down(absolute, ALIGNMENT);
        let preserve_end = align_up(absolute + data.len() as u64, ALIGNMENT)
            .min(self.base_offset + self.logical_len());
        let buffer_end = absolute + data.len() as u64;
        let write_end = buffer_end.max(preserve_end);
        let mut encrypted = vec![0u8; (write_end - sector_start) as usize];
        let existing_len = ((self.base_offset + self.logical_len()).saturating_sub(sector_start))
            .min(write_end - sector_start);
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("virtual disk mutex poisoned"))?;
        if existing_len > 0 {
            guard.file.seek(SeekFrom::Start(sector_start))?;
            guard
                .file
                .read_exact(&mut encrypted[..existing_len as usize])?;
            self.apply_xts(sector_start, &mut encrypted[..existing_len as usize], false);
        }
        let relative = (absolute - sector_start) as usize;
        encrypted[relative..relative + data.len()].copy_from_slice(data);
        self.apply_xts(sector_start, &mut encrypted, true);
        guard.file.seek(SeekFrom::Start(sector_start))?;
        guard.file.write_all(&encrypted)?;
        if end > self.logical_len() {
            self.set_len_locked(&mut guard, end)?;
        }
        Ok(())
    }
}

fn align_up(value: u64, alignment: u64) -> u64 {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + (alignment - remainder)
    }
}

fn align_down(value: u64, alignment: u64) -> u64 {
    value / alignment * alignment
}

fn file_descriptor(file: &File) -> String {
    file.metadata()
        .map(|meta| format!("virtual disk ({} bytes)", meta.len()))
        .unwrap_or_else(|_| "virtual disk".to_string())
}

pub fn blob_volume_name(namespace: &str, key: &str) -> String {
    let namespace = sanitize_volume_component(namespace);
    let key = sanitize_volume_component(key);
    format!("blob.{namespace}.{key}")
}

pub fn has_volume(disk_path: &Path, volume_name: &str) -> Result<bool> {
    if !disk_path.exists() {
        return Ok(false);
    }

    let mut file = OpenOptions::new()
        .read(true)
        .open(disk_path)
        .with_context(|| format!("failed to open virtual disk {}", disk_path.display()))?;
    let manifest = VolumeBackend::load_or_initialize_manifest(&mut file)?;
    Ok(manifest.volumes.contains_key(volume_name))
}

pub fn volume_info(disk_path: &Path, volume_name: &str) -> Result<Option<VolumeInfo>> {
    if !disk_path.exists() {
        return Ok(None);
    }

    let mut file = OpenOptions::new()
        .read(true)
        .open(disk_path)
        .with_context(|| format!("failed to open virtual disk {}", disk_path.display()))?;
    let manifest = VolumeBackend::load_or_initialize_manifest(&mut file)?;
    Ok(manifest.volumes.get(volume_name).map(|record| VolumeInfo {
        offset: record.offset,
        capacity: record.capacity,
        len: record.len,
    }))
}

pub fn read_volume_prefix(
    disk_path: &Path,
    volume_name: &str,
    len: usize,
) -> Result<Option<Vec<u8>>> {
    if len == 0 {
        return Ok(Some(Vec::new()));
    }
    let Some(info) = volume_info(disk_path, volume_name)? else {
        return Ok(None);
    };
    if info.len == 0 {
        return Ok(Some(Vec::new()));
    }

    let to_read =
        usize::try_from(info.len.min(len as u64)).context("volume prefix length overflow")?;
    let mut file = OpenOptions::new()
        .read(true)
        .open(disk_path)
        .with_context(|| format!("failed to open virtual disk {}", disk_path.display()))?;
    file.seek(SeekFrom::Start(info.offset))?;
    let mut buf = vec![0u8; to_read];
    file.read_exact(&mut buf)?;
    Ok(Some(buf))
}

pub fn import_file(
    disk_path: &Path,
    volume_name: &str,
    source_path: &Path,
    key: Option<[u8; 32]>,
) -> Result<()> {
    let bytes = std::fs::read(source_path)
        .with_context(|| format!("failed to read blob source {}", source_path.display()))?;
    import_bytes(disk_path, volume_name, &bytes, key)
}

pub fn replace_file(
    disk_path: &Path,
    volume_name: &str,
    source_path: &Path,
    key: Option<[u8; 32]>,
) -> Result<()> {
    let bytes = std::fs::read(source_path)
        .with_context(|| format!("failed to read blob source {}", source_path.display()))?;
    replace_bytes(disk_path, volume_name, &bytes, key)
}

pub fn import_bytes(
    disk_path: &Path,
    volume_name: &str,
    bytes: &[u8],
    key: Option<[u8; 32]>,
) -> Result<()> {
    let capacity = align_up(bytes.len() as u64 + ALIGNMENT, ALIGNMENT);
    let backend = VolumeBackend::open(disk_path, volume_name, capacity, key)?;
    backend.write_all(bytes)?;
    Ok(())
}

pub fn replace_bytes(
    disk_path: &Path,
    volume_name: &str,
    bytes: &[u8],
    key: Option<[u8; 32]>,
) -> Result<()> {
    let capacity = align_up(bytes.len() as u64 + ALIGNMENT, ALIGNMENT);

    if let Some(parent) = disk_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create parent directory for {}",
                disk_path.display()
            )
        })?;
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(disk_path)
        .with_context(|| format!("failed to open virtual disk {}", disk_path.display()))?;

    let mut manifest = VolumeBackend::load_or_initialize_manifest(&mut file)?;
    manifest.volumes.remove(volume_name);

    let next_offset = manifest
        .volumes
        .values()
        .map(|v| v.offset + v.capacity)
        .max()
        .unwrap_or(HEADER_BYTES);
    let offset = align_up(next_offset, ALIGNMENT);
    manifest.volumes.insert(
        volume_name.to_string(),
        VolumeRecord {
            offset,
            capacity,
            len: 0,
        },
    );
    VolumeBackend::persist_manifest(&mut file, &manifest)?;
    let required_len = offset + capacity;
    if file.metadata()?.len() < required_len {
        file.set_len(required_len)?;
    }
    drop(file);

    let backend = VolumeBackend::open(disk_path, volume_name, capacity, key)?;
    backend.write_all(bytes)?;
    Ok(())
}

pub fn remove_volume(disk_path: &Path, volume_name: &str) -> Result<bool> {
    if !disk_path.exists() {
        return Ok(false);
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(disk_path)
        .with_context(|| format!("failed to open virtual disk {}", disk_path.display()))?;
    let mut manifest = VolumeBackend::load_or_initialize_manifest(&mut file)?;
    let removed = manifest.volumes.remove(volume_name).is_some();
    if removed {
        VolumeBackend::persist_manifest(&mut file, &manifest)?;
    }
    Ok(removed)
}

pub fn materialize_file(
    disk_path: &Path,
    volume_name: &str,
    destination_path: &Path,
    key: Option<[u8; 32]>,
) -> Result<PathBuf> {
    let backend = VolumeBackend::open(disk_path, volume_name, ALIGNMENT, key)?;
    let bytes = backend.read_all()?;
    if let Some(parent) = destination_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create blob materialization directory {}",
                parent.display()
            )
        })?;
    }
    std::fs::write(destination_path, bytes).with_context(|| {
        format!(
            "failed to materialize blob volume '{volume_name}' to {}",
            destination_path.display()
        )
    })?;
    Ok(destination_path.to_path_buf())
}

fn sanitize_volume_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "blob".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{has_volume, import_bytes, VolumeBackend, ALIGNMENT};
    use redb::Database;
    use std::io::{Read, Seek, SeekFrom};
    use tempfile::tempdir;

    #[test]
    fn volume_backend_round_trips_multiple_volumes() {
        let dir = tempdir().unwrap();
        let disk = dir.path().join("rockbot.data");

        let sessions = VolumeBackend::open(&disk, "sessions", 64 * 1024, None).unwrap();
        let cron = VolumeBackend::open(&disk, "cron", 64 * 1024, None).unwrap();

        sessions.write_all(b"session-bytes").unwrap();
        cron.write_all(b"cron-bytes").unwrap();
        assert_eq!(sessions.read_all().unwrap(), b"session-bytes");
        assert_eq!(cron.read_all().unwrap(), b"cron-bytes");
    }

    #[test]
    fn encrypted_volume_backend_round_trips() {
        let dir = tempdir().unwrap();
        let disk = dir.path().join("rockbot.data");
        let key = [7u8; 32];

        let backend = VolumeBackend::open(&disk, "agents", 2 * 1024 * 1024, Some(key)).unwrap();
        let db = Database::builder().create_with_backend(backend).unwrap();
        let tx = db.begin_write().unwrap();
        {
            let mut table = tx
                .open_table(redb::TableDefinition::<&str, &[u8]>::new("agents"))
                .unwrap();
            table.insert("hex", b"alive".as_slice()).unwrap();
        }
        tx.commit().unwrap();

        let bytes = std::fs::read(&disk).unwrap();
        assert!(!bytes.windows(5).any(|window| window == b"alive"));
    }

    #[test]
    fn encrypted_volumes_do_not_share_ciphertext() {
        let dir = tempdir().unwrap();
        let disk = dir.path().join("rockbot.data");
        let key = [9u8; 32];

        let agents = VolumeBackend::open(&disk, "agents", 64 * 1024, Some(key)).unwrap();
        let sessions = VolumeBackend::open(&disk, "sessions", 64 * 1024, Some(key)).unwrap();

        agents.write_all(&vec![b'A'; ALIGNMENT as usize]).unwrap();
        sessions.write_all(&vec![b'A'; ALIGNMENT as usize]).unwrap();

        let mut file = std::fs::File::open(&disk).unwrap();
        let mut agents_bytes = vec![0u8; ALIGNMENT as usize];
        let mut sessions_bytes = vec![0u8; ALIGNMENT as usize];
        file.seek(SeekFrom::Start(agents.base_offset)).unwrap();
        file.read_exact(&mut agents_bytes).unwrap();
        file.seek(SeekFrom::Start(sessions.base_offset)).unwrap();
        file.read_exact(&mut sessions_bytes).unwrap();

        assert_ne!(agents_bytes, sessions_bytes);
    }

    #[test]
    fn imported_legacy_redb_file_opens_as_volume() {
        let dir = tempdir().unwrap();
        let source_disk = dir.path().join("source.data");
        let target_disk = dir.path().join("rockbot.data");

        let source_backend =
            VolumeBackend::open(&source_disk, "legacy-src", 2 * 1024 * 1024, None).unwrap();
        let db = Database::builder()
            .create_with_backend(source_backend)
            .unwrap();
        let tx = db.begin_write().unwrap();
        {
            let mut table = tx
                .open_table(redb::TableDefinition::<&str, &[u8]>::new("legacy"))
                .unwrap();
            table.insert("hello", b"world".as_slice()).unwrap();
        }
        tx.commit().unwrap();
        drop(db);

        let bytes = VolumeBackend::open(&source_disk, "legacy-src", 2 * 1024 * 1024, None)
            .unwrap()
            .read_all()
            .unwrap();
        import_bytes(&target_disk, "vault", &bytes, None).unwrap();
        assert!(has_volume(&target_disk, "vault").unwrap());

        let backend = VolumeBackend::open(&target_disk, "vault", 2 * 1024 * 1024, None).unwrap();
        let db = Database::builder().create_with_backend(backend).unwrap();
        let tx = db.begin_read().unwrap();
        let table = tx
            .open_table(redb::TableDefinition::<&str, &[u8]>::new("legacy"))
            .unwrap();
        let value = table.get("hello").unwrap().unwrap();
        assert_eq!(value.value(), b"world");
    }
}
