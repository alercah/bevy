use bevy_asset::{io::Reader, AssetLoader, AssetServer, AsyncReadExt, LoadContext};
use bevy_ecs::prelude::{FromWorld, World};
use thiserror::Error;

use crate::{
    render_asset::RenderAssetPersistencePolicy,
    renderer::RenderDevice,
    texture::{Image, ImageFormat, ImageType, TextureError},
};

use super::{CompressedImageFormats, ImageSampler};
use serde::{Deserialize, Serialize};

/// Loader for images that can be read by the `image` crate.
#[derive(Clone)]
pub struct ImageLoader {
    supported_compressed_formats: CompressedImageFormats,
}

pub(crate) const IMG_FILE_EXTENSIONS: &[&str] = &[
    #[cfg(feature = "basis-universal")]
    "basis",
    #[cfg(feature = "bmp")]
    "bmp",
    #[cfg(feature = "png")]
    "png",
    #[cfg(feature = "dds")]
    "dds",
    #[cfg(feature = "tga")]
    "tga",
    #[cfg(feature = "jpeg")]
    "jpg",
    #[cfg(feature = "jpeg")]
    "jpeg",
    #[cfg(feature = "ktx2")]
    "ktx2",
    #[cfg(feature = "webp")]
    "webp",
    #[cfg(feature = "pnm")]
    "pam",
    #[cfg(feature = "pnm")]
    "pbm",
    #[cfg(feature = "pnm")]
    "pgm",
    #[cfg(feature = "pnm")]
    "ppm",
];

pub(crate) struct DisabledExtension {
    extension: &'static str,
    feature: &'static str,
}

macro_rules! disabled_ext (
    ($feature:literal, $ext:literal) => (
        #[cfg(not(feature = $feature))]
        DisabledExtension{
            extension: $ext,
            feature: $feature,
        }
    );
);

pub(crate) const DISABLED_IMG_FILE_EXTENSIONS: &[DisabledExtension] = &[
    disabled_ext!("basis-universal", "basis"),
    disabled_ext!("bmp", "bmp"),
    disabled_ext!("png", "png"),
    disabled_ext!("dds", "dds"),
    disabled_ext!("tga", "tga"),
    disabled_ext!("jpeg", "jpg"),
    disabled_ext!("jpeg", "jpeg"),
    disabled_ext!("ktx2", "ktx2"),
    disabled_ext!("webp", "webp"),
    disabled_ext!("pnm", "pam"),
    disabled_ext!("pnm", "pbm"),
    disabled_ext!("pnm", "pgm"),
    disabled_ext!("pnm", "ppm"),
];

#[derive(Serialize, Deserialize, Default, Debug)]
pub enum ImageFormatSetting {
    #[default]
    FromExtension,
    Format(ImageFormat),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageLoaderSettings {
    pub format: ImageFormatSetting,
    pub is_srgb: bool,
    pub sampler: ImageSampler,
    pub cpu_persistent_access: RenderAssetPersistencePolicy,
}

impl Default for ImageLoaderSettings {
    fn default() -> Self {
        Self {
            format: ImageFormatSetting::default(),
            is_srgb: true,
            sampler: ImageSampler::Default,
            cpu_persistent_access: RenderAssetPersistencePolicy::Keep,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ImageLoaderError {
    #[error("Could load shader: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not load texture file: {0}")]
    FileTexture(#[from] FileTextureError),
}

impl AssetLoader for ImageLoader {
    type Asset = Image;
    type Settings = ImageLoaderSettings;
    type Error = ImageLoaderError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a ImageLoaderSettings,
        load_context: &'a mut LoadContext,
    ) -> bevy_utils::BoxedFuture<'a, Result<Image, Self::Error>> {
        Box::pin(async move {
            // use the file extension for the image type
            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let image_type = match settings.format {
                ImageFormatSetting::FromExtension => ImageType::Extension(ext),
                ImageFormatSetting::Format(format) => ImageType::Format(format),
            };
            Ok(Image::from_buffer(
                &bytes,
                image_type,
                self.supported_compressed_formats,
                settings.is_srgb,
                settings.sampler.clone(),
                settings.cpu_persistent_access,
            )
            .map_err(|err| FileTextureError {
                error: err,
                path: format!("{}", load_context.path().display()),
            })?)
        })
    }

    fn extensions(&self) -> &[&str] {
        IMG_FILE_EXTENSIONS
    }
}

impl FromWorld for ImageLoader {
    fn from_world(world: &mut World) -> Self {
        let supported_compressed_formats = match world.get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),

            None => CompressedImageFormats::NONE,
        };

        if let Some(asset_server) = world.get_resource::<AssetServer>() {
            for DisabledExtension { extension, feature } in DISABLED_IMG_FILE_EXTENSIONS {
                asset_server.register_extension_hint(
                    extension,
                    format!("enabling bevy feature '{}'", feature),
                )
            }
        }

        Self {
            supported_compressed_formats,
        }
    }
}

/// An error that occurs when loading a texture from a file.
#[derive(Error, Debug)]
pub struct FileTextureError {
    error: TextureError,
    path: String,
}
impl std::fmt::Display for FileTextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "Error reading image file {}: {}, this is an error in `bevy_render`.",
            self.path, self.error
        )
    }
}
