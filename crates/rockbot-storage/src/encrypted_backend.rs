use aes::cipher::KeyInit;
use aes::Aes256;
use redb::StorageBackend;
use ring::hkdf;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;
use xts_mode::{get_tweak_default, Xts128};

const XTS_SECTOR_SIZE: usize = 4096;
const XTS_KEY_INFO: &[u8] = b"rockbot-storage/aes-256-xts/v1";

/// A redb `StorageBackend` that transparently encrypts data using AES-256-XTS.
pub struct EncryptedBackend {
    inner: Mutex<File>,
    key: [u8; 64],
}

impl fmt::Debug for EncryptedBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncryptedBackend")
            .field("key", &"[redacted]")
            .finish()
    }
}

struct XtsKeyLen;

impl hkdf::KeyType for XtsKeyLen {
    fn len(&self) -> usize {
        64
    }
}

impl EncryptedBackend {
    /// Open (or create) an encrypted file at `path` with the given 32-byte root key.
    pub fn open(path: &Path, key: [u8; 32]) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        Ok(Self {
            inner: Mutex::new(file),
            key: Self::derive_xts_key(&key, XTS_KEY_INFO)?,
        })
    }

    fn derive_xts_key(root_key: &[u8; 32], info: &[u8]) -> io::Result<[u8; 64]> {
        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"rockbot-storage-root-key");
        let prk = salt.extract(root_key);
        let info_refs = [info];
        let okm = prk
            .expand(&info_refs, XtsKeyLen)
            .map_err(|_| io::Error::other("invalid AES-XTS key derivation context"))?;
        let mut derived = [0u8; 64];
        okm.fill(&mut derived)
            .map_err(|_| io::Error::other("failed to derive AES-XTS key"))?;
        Ok(derived)
    }

    fn xts_cipher(&self) -> Xts128<Aes256> {
        let cipher_1 = Aes256::new_from_slice(&self.key[..32]).expect("valid AES-256 key");
        let cipher_2 = Aes256::new_from_slice(&self.key[32..]).expect("valid AES-256 key");
        Xts128::new(cipher_1, cipher_2)
    }

    fn align_down(offset: u64) -> u64 {
        offset / XTS_SECTOR_SIZE as u64 * XTS_SECTOR_SIZE as u64
    }

    fn align_up(offset: u64) -> u64 {
        if offset == 0 {
            return 0;
        }
        let sector = XTS_SECTOR_SIZE as u64;
        offset.div_ceil(sector) * sector
    }

    fn decrypt_region(&self, start: u64, data: &mut [u8]) {
        if data.is_empty() {
            return;
        }
        self.xts_cipher().decrypt_area(
            data,
            XTS_SECTOR_SIZE,
            u128::from(start / XTS_SECTOR_SIZE as u64),
            get_tweak_default,
        );
    }

    fn encrypt_region(&self, start: u64, data: &mut [u8]) {
        if data.is_empty() {
            return;
        }
        self.xts_cipher().encrypt_area(
            data,
            XTS_SECTOR_SIZE,
            u128::from(start / XTS_SECTOR_SIZE as u64),
            get_tweak_default,
        );
    }
}

impl StorageBackend for EncryptedBackend {
    fn len(&self) -> Result<u64, io::Error> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("mutex poisoned"))?;
        Ok(guard.metadata()?.len())
    }

    fn read(&self, offset: u64, len: usize) -> Result<Vec<u8>, io::Error> {
        let len_u64 = u64::try_from(len).map_err(|_| io::Error::other("length overflow"))?;
        let start = Self::align_down(offset);
        let end = Self::align_up(
            offset
                .checked_add(len_u64)
                .ok_or_else(|| io::Error::other("offset overflow"))?,
        );
        let file_len = self.len()?;
        let read_end = end.min(file_len);
        let read_len = usize::try_from(read_end.saturating_sub(start))
            .map_err(|_| io::Error::other("length overflow"))?;
        let mut buf = vec![0u8; read_len];

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("mutex poisoned"))?;
        guard.seek(SeekFrom::Start(start))?;
        guard.read_exact(&mut buf)?;
        drop(guard);

        self.decrypt_region(start, &mut buf);
        let relative = usize::try_from(offset.saturating_sub(start))
            .map_err(|_| io::Error::other("offset overflow"))?;
        Ok(buf[relative..relative + len].to_vec())
    }

    fn set_len(&self, len: u64) -> Result<(), io::Error> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("mutex poisoned"))?;
        guard.set_len(len)
    }

    fn sync_data(&self, _eventual: bool) -> Result<(), io::Error> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("mutex poisoned"))?;
        guard.sync_data()
    }

    fn write(&self, offset: u64, data: &[u8]) -> Result<(), io::Error> {
        if data.is_empty() {
            return Ok(());
        }

        let write_end = offset
            .checked_add(
                u64::try_from(data.len()).map_err(|_| io::Error::other("length overflow"))?,
            )
            .ok_or_else(|| io::Error::other("offset overflow"))?;
        let sector_start = Self::align_down(offset);
        let current_len = self.len()?;
        let preserve_end = Self::align_up(write_end).min(current_len);
        let buffer_end = write_end.max(preserve_end);
        let buffer_len = usize::try_from(buffer_end.saturating_sub(sector_start))
            .map_err(|_| io::Error::other("length overflow"))?;
        let existing_len = usize::try_from(
            current_len
                .saturating_sub(sector_start)
                .min(u64::try_from(buffer_len).map_err(|_| io::Error::other("length overflow"))?),
        )
        .map_err(|_| io::Error::other("length overflow"))?;

        let mut buffer = vec![0u8; buffer_len];
        if existing_len > 0 {
            let mut guard = self
                .inner
                .lock()
                .map_err(|_| io::Error::other("mutex poisoned"))?;
            guard.seek(SeekFrom::Start(sector_start))?;
            guard.read_exact(&mut buffer[..existing_len])?;
            drop(guard);
            self.decrypt_region(sector_start, &mut buffer[..existing_len]);
        }

        let relative = usize::try_from(offset.saturating_sub(sector_start))
            .map_err(|_| io::Error::other("offset overflow"))?;
        buffer[relative..relative + data.len()].copy_from_slice(data);
        self.encrypt_region(sector_start, &mut buffer);

        let mut guard = self
            .inner
            .lock()
            .map_err(|_| io::Error::other("mutex poisoned"))?;
        guard.seek(SeekFrom::Start(sector_start))?;
        guard.write_all(&buffer)
    }
}
