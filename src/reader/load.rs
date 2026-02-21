use std::convert::TryInto;
#[cfg(not(feature = "async"))]
use std::fs::File;
#[cfg(not(feature = "async"))]
use std::io::Read;
use std::path::Path;

#[cfg(feature = "async")]
use tokio::fs::File;
#[cfg(feature = "async")]
use tokio::io::{AsyncRead, AsyncReadExt};
#[cfg(feature = "async")]
use tokio::pin;

use super::{FilterFunc, PdfMetadata, Reader};
use crate::{Document, Error, IncrementalDocument, Result};

#[cfg(not(feature = "async"))]
impl Document {
    /// Load a PDF document from a specified file path.
    #[inline]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Document> {
        let file = File::open(path)?;
        let capacity = Some(file.metadata()?.len() as usize);
        Self::load_internal(file, capacity, None, None)
    }

    /// Load a PDF document from a specified file path with a password for encrypted PDFs.
    #[inline]
    pub fn load_with_password<P: AsRef<Path>>(path: P, password: &str) -> Result<Document> {
        let file = File::open(path)?;
        let capacity = Some(file.metadata()?.len() as usize);
        Self::load_internal(file, capacity, None, Some(password.to_string()))
    }

    #[inline]
    pub fn load_filtered<P: AsRef<Path>>(path: P, filter_func: FilterFunc) -> Result<Document> {
        let file = File::open(path)?;
        let capacity = Some(file.metadata()?.len() as usize);
        Self::load_internal(file, capacity, Some(filter_func), None)
    }

    /// Load a PDF document from an arbitrary source.
    #[inline]
    pub fn load_from<R: Read>(source: R) -> Result<Document> {
        Self::load_internal(source, None, None, None)
    }

    /// Load a PDF document from an arbitrary source with a password for encrypted PDFs.
    #[inline]
    pub fn load_from_with_password<R: Read>(source: R, password: &str) -> Result<Document> {
        Self::load_internal(source, None, None, Some(password.to_string()))
    }

    fn load_internal<R: Read>(
        mut source: R, capacity: Option<usize>, filter_func: Option<FilterFunc>, password: Option<String>,
    ) -> Result<Document> {
        let mut buffer = capacity.map(Vec::with_capacity).unwrap_or_default();
        source.read_to_end(&mut buffer)?;

        Reader {
            buffer: &buffer,
            document: Document::new(),
            encryption_state: None,

            password,
        }
        .read(filter_func)
    }

    /// Load a PDF document from a memory slice.
    pub fn load_mem(buffer: &[u8]) -> Result<Document> {
        buffer.try_into()
    }

    /// Load a PDF document from a memory slice with a password for encrypted PDFs.
    pub fn load_mem_with_password(buffer: &[u8], password: &str) -> Result<Document> {
        Reader {
            buffer,
            document: Document::new(),
            encryption_state: None,

            password: Some(password.to_string()),
        }
        .read(None)
    }

    /// Load PDF metadata (title and page count) without loading the entire document.
    /// This is much faster for large PDFs when you only need basic information.
    #[inline]
    pub fn load_metadata<P: AsRef<Path>>(path: P) -> Result<PdfMetadata> {
        let file = File::open(path)?;
        let capacity = Some(file.metadata()?.len() as usize);
        Self::load_metadata_internal(file, capacity, None)
    }

    /// Load PDF metadata from a file path with a password for encrypted PDFs.
    #[inline]
    pub fn load_metadata_with_password<P: AsRef<Path>>(path: P, password: &str) -> Result<PdfMetadata> {
        let file = File::open(path)?;
        let capacity = Some(file.metadata()?.len() as usize);
        Self::load_metadata_internal(file, capacity, Some(password.to_string()))
    }

    /// Load PDF metadata from an arbitrary source without loading the entire document.
    #[inline]
    pub fn load_metadata_from<R: Read>(source: R) -> Result<PdfMetadata> {
        Self::load_metadata_internal(source, None, None)
    }

    /// Load PDF metadata from an arbitrary source with a password for encrypted PDFs.
    #[inline]
    pub fn load_metadata_from_with_password<R: Read>(source: R, password: &str) -> Result<PdfMetadata> {
        Self::load_metadata_internal(source, None, Some(password.to_string()))
    }

    /// Load PDF metadata from a memory slice without loading the entire document.
    #[inline]
    pub fn load_metadata_mem(buffer: &[u8]) -> Result<PdfMetadata> {
        Reader {
            buffer,
            document: Document::new(),
            encryption_state: None,

            password: None,
        }
        .read_metadata()
    }

    /// Load PDF metadata from a memory slice with a password for encrypted PDFs.
    #[inline]
    pub fn load_metadata_mem_with_password(buffer: &[u8], password: &str) -> Result<PdfMetadata> {
        Reader {
            buffer,
            document: Document::new(),
            encryption_state: None,

            password: Some(password.to_string()),
        }
        .read_metadata()
    }

    fn load_metadata_internal<R: Read>(
        mut source: R, capacity: Option<usize>, password: Option<String>,
    ) -> Result<PdfMetadata> {
        let mut buffer = capacity.map(Vec::with_capacity).unwrap_or_default();
        source.read_to_end(&mut buffer)?;

        Reader {
            buffer: &buffer,
            document: Document::new(),
            encryption_state: None,

            password,
        }
        .read_metadata()
    }
}

#[cfg(feature = "async")]
impl Document {
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Document> {
        let file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let capacity = Some(metadata.len() as usize);
        Self::load_internal(file, capacity, None, None).await
    }

    /// Load a PDF document from a specified file path with a password for encrypted PDFs.
    pub async fn load_with_password<P: AsRef<Path>>(path: P, password: &str) -> Result<Document> {
        let file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let capacity = Some(metadata.len() as usize);
        Self::load_internal(file, capacity, None, Some(password.to_string())).await
    }

    pub async fn load_filtered<P: AsRef<Path>>(path: P, filter_func: FilterFunc) -> Result<Document> {
        let file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let capacity = Some(metadata.len() as usize);
        Self::load_internal(file, capacity, Some(filter_func), None).await
    }

    async fn load_internal<R: AsyncRead>(
        source: R, capacity: Option<usize>, filter_func: Option<FilterFunc>, password: Option<String>,
    ) -> Result<Document> {
        pin!(source);

        let mut buffer = capacity.map(Vec::with_capacity).unwrap_or_default();
        source.read_to_end(&mut buffer).await?;

        Reader {
            buffer: &buffer,
            document: Document::new(),
            encryption_state: None,

            password,
        }
        .read(filter_func)
    }

    /// Load a PDF document from a memory slice.
    pub fn load_mem(buffer: &[u8]) -> Result<Document> {
        buffer.try_into()
    }

    /// Load PDF metadata (title and page count) without loading the entire document.
    /// This is much faster for large PDFs when you only need basic information.
    #[inline]
    pub async fn load_metadata<P: AsRef<Path>>(path: P) -> Result<PdfMetadata> {
        let file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let capacity = Some(metadata.len() as usize);
        Self::load_metadata_internal(file, capacity, None).await
    }

    /// Load PDF metadata from a file path with a password for encrypted PDFs.
    #[inline]
    pub async fn load_metadata_with_password<P: AsRef<Path>>(path: P, password: &str) -> Result<PdfMetadata> {
        let file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let capacity = Some(metadata.len() as usize);
        Self::load_metadata_internal(file, capacity, Some(password.to_string())).await
    }

    /// Load PDF metadata from an arbitrary source without loading the entire document.
    #[inline]
    pub async fn load_metadata_from<R: AsyncRead>(source: R) -> Result<PdfMetadata> {
        Self::load_metadata_internal(source, None, None).await
    }

    /// Load PDF metadata from an arbitrary source with a password for encrypted PDFs.
    #[inline]
    pub async fn load_metadata_from_with_password<R: AsyncRead>(source: R, password: &str) -> Result<PdfMetadata> {
        Self::load_metadata_internal(source, None, Some(password.to_string())).await
    }

    /// Load PDF metadata from a memory slice without loading the entire document.
    #[inline]
    pub fn load_metadata_mem(buffer: &[u8]) -> Result<PdfMetadata> {
        Reader {
            buffer,
            document: Document::new(),
            encryption_state: None,

            password: None,
        }
        .read_metadata()
    }

    /// Load PDF metadata from a memory slice with a password for encrypted PDFs.
    #[inline]
    pub fn load_metadata_mem_with_password(buffer: &[u8], password: &str) -> Result<PdfMetadata> {
        Reader {
            buffer,
            document: Document::new(),
            encryption_state: None,

            password: Some(password.to_string()),
        }
        .read_metadata()
    }

    async fn load_metadata_internal<R: AsyncRead>(
        source: R, capacity: Option<usize>, password: Option<String>,
    ) -> Result<PdfMetadata> {
        pin!(source);

        let mut buffer = capacity.map(Vec::with_capacity).unwrap_or_default();
        source.read_to_end(&mut buffer).await?;

        Reader {
            buffer: &buffer,
            document: Document::new(),
            encryption_state: None,

            password,
        }
        .read_metadata()
    }
}

impl TryInto<Document> for &[u8] {
    type Error = Error;

    fn try_into(self) -> Result<Document> {
        Reader {
            buffer: self,
            document: Document::new(),
            encryption_state: None,

            password: None,
        }
        .read(None)
    }
}

#[cfg(not(feature = "async"))]
impl IncrementalDocument {
    /// Load a PDF document from a specified file path.
    #[inline]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let capacity = Some(file.metadata()?.len() as usize);
        Self::load_internal(file, capacity)
    }

    /// Load a PDF document from an arbitrary source.
    #[inline]
    pub fn load_from<R: Read>(source: R) -> Result<Self> {
        Self::load_internal(source, None)
    }

    fn load_internal<R: Read>(mut source: R, capacity: Option<usize>) -> Result<Self> {
        let mut buffer = capacity.map(Vec::with_capacity).unwrap_or_default();
        source.read_to_end(&mut buffer)?;

        let document = Reader {
            buffer: &buffer,
            document: Document::new(),
            encryption_state: None,

            password: None,
        }
        .read(None)?;

        Ok(IncrementalDocument::create_from(buffer, document))
    }

    /// Load a PDF document from a memory slice.
    pub fn load_mem(buffer: &[u8]) -> Result<Document> {
        buffer.try_into()
    }
}

#[cfg(feature = "async")]
impl IncrementalDocument {
    /// Load a PDF document from a specified file path.
    #[inline]
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).await?;
        let metadata = file.metadata().await?;
        let capacity = Some(metadata.len() as usize);
        Self::load_internal(file, capacity).await
    }

    /// Load a PDF document from an arbitrary source.
    #[inline]
    pub async fn load_from<R: AsyncRead>(source: R) -> Result<Self> {
        Self::load_internal(source, None).await
    }

    async fn load_internal<R: AsyncRead>(source: R, capacity: Option<usize>) -> Result<Self> {
        pin!(source);

        let mut buffer = capacity.map(Vec::with_capacity).unwrap_or_default();
        source.read_to_end(&mut buffer).await?;

        let document = Reader {
            buffer: &buffer,
            document: Document::new(),
            encryption_state: None,

            password: None,
        }
        .read(None)?;

        Ok(IncrementalDocument::create_from(buffer, document))
    }

    /// Load a PDF document from a memory slice.
    pub fn load_mem(buffer: &[u8]) -> Result<Document> {
        buffer.try_into()
    }
}

impl TryInto<IncrementalDocument> for &[u8] {
    type Error = Error;

    fn try_into(self) -> Result<IncrementalDocument> {
        let document = Reader {
            buffer: self,
            document: Document::new(),
            encryption_state: None,

            password: None,
        }
        .read(None)?;

        Ok(IncrementalDocument::create_from(self.to_vec(), document))
    }
}
