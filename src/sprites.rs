//! Chargement des images (PNG embarqués) en bitmaps Direct2D.
use crate::prelude::*;

pub(crate) struct Sprites {
    pub(crate) deer: [ID2D1Bitmap1; 4],
    pub(crate) sleigh: ID2D1Bitmap1,
    pub(crate) trees: [ID2D1Bitmap1; 3],
    /// Personnages et objets qui se tiennent debout (bonhommes de neige, pingouin, ours, cadeaux...).
    pub(crate) chars: Vec<ID2D1Bitmap1>,
    pub(crate) santa_walk: Vec<ID2D1Bitmap1>,
    pub(crate) santa_wave: ID2D1Bitmap1,
    /// Collection d'icônes (40 éléments, voir `Src::Ico`).
    pub(crate) ico: Vec<ID2D1Bitmap1>,
}

pub(crate) fn load_bitmap(ctx: &ID2D1DeviceContext, wic: &IWICImagingFactory, data: &[u8]) -> Result<ID2D1Bitmap1> {
    unsafe {
        let stream = wic.CreateStream()?;
        stream.InitializeFromMemory(data)?;
        let dec = wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
        let frame = dec.GetFrame(0)?;
        let conv = wic.CreateFormatConverter()?;
        conv.Initialize(&frame, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom)?;
        ctx.CreateBitmapFromWicBitmap(&conv, None)
    }
}

impl Sprites {
    pub(crate) unsafe fn load(ctx: &ID2D1DeviceContext) -> Result<Sprites> {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let l = |d: &[u8]| load_bitmap(ctx, &wic, d);
        Ok(Sprites {
            deer: [
                l(include_bytes!("../assets/reindeer_1.png"))?,
                l(include_bytes!("../assets/reindeer_2.png"))?,
                l(include_bytes!("../assets/reindeer_3.png"))?,
                l(include_bytes!("../assets/reindeer_4.png"))?,
            ],
            sleigh: l(include_bytes!("../assets/sleigh.png"))?,
            trees: [
                l(include_bytes!("../assets/tree_1.png"))?,
                l(include_bytes!("../assets/tree_2.png"))?,
                l(include_bytes!("../assets/tree_3.png"))?,
            ],
            santa_walk: vec![
                l(include_bytes!("../assets/santa_walk_1.png"))?,
                l(include_bytes!("../assets/santa_walk_2.png"))?,
                l(include_bytes!("../assets/santa_walk_3.png"))?,
                l(include_bytes!("../assets/santa_walk_4.png"))?,
                l(include_bytes!("../assets/santa_walk_5.png"))?,
                l(include_bytes!("../assets/santa_walk_6.png"))?,
                l(include_bytes!("../assets/santa_walk_7.png"))?,
                l(include_bytes!("../assets/santa_walk_8.png"))?,
            ],
            santa_wave: l(include_bytes!("../assets/santa_wave.png"))?,
            ico: vec![
                l(include_bytes!("../assets/ico_01.png"))?,
                l(include_bytes!("../assets/ico_02.png"))?,
                l(include_bytes!("../assets/ico_03.png"))?,
                l(include_bytes!("../assets/ico_04.png"))?,
                l(include_bytes!("../assets/ico_05.png"))?,
                l(include_bytes!("../assets/ico_06.png"))?,
                l(include_bytes!("../assets/ico_07.png"))?,
                l(include_bytes!("../assets/ico_08.png"))?,
                l(include_bytes!("../assets/ico_09.png"))?,
                l(include_bytes!("../assets/ico_10.png"))?,
                l(include_bytes!("../assets/ico_11.png"))?,
                l(include_bytes!("../assets/ico_12.png"))?,
                l(include_bytes!("../assets/ico_13.png"))?,
                l(include_bytes!("../assets/ico_14.png"))?,
                l(include_bytes!("../assets/ico_15.png"))?,
                l(include_bytes!("../assets/ico_16.png"))?,
                l(include_bytes!("../assets/ico_17.png"))?,
                l(include_bytes!("../assets/ico_18.png"))?,
                l(include_bytes!("../assets/ico_19.png"))?,
                l(include_bytes!("../assets/ico_20.png"))?,
                l(include_bytes!("../assets/ico_21.png"))?,
                l(include_bytes!("../assets/ico_22.png"))?,
                l(include_bytes!("../assets/ico_23.png"))?,
                l(include_bytes!("../assets/ico_24.png"))?,
                l(include_bytes!("../assets/ico_25.png"))?,
                l(include_bytes!("../assets/ico_26.png"))?,
                l(include_bytes!("../assets/ico_27.png"))?,
                l(include_bytes!("../assets/ico_28.png"))?,
                l(include_bytes!("../assets/ico_29.png"))?,
                l(include_bytes!("../assets/ico_30.png"))?,
                l(include_bytes!("../assets/ico_31.png"))?,
                l(include_bytes!("../assets/ico_32.png"))?,
                l(include_bytes!("../assets/ico_33.png"))?,
                l(include_bytes!("../assets/ico_34.png"))?,
                l(include_bytes!("../assets/ico_35.png"))?,
                l(include_bytes!("../assets/ico_36.png"))?,
                l(include_bytes!("../assets/ico_37.png"))?,
                l(include_bytes!("../assets/ico_38.png"))?,
                l(include_bytes!("../assets/ico_39.png"))?,
                l(include_bytes!("../assets/ico_40.png"))?,
            ],
            chars: vec![
                l(include_bytes!("../assets/ch_p1.png"))?,
                l(include_bytes!("../assets/ch_p2.png"))?,
                l(include_bytes!("../assets/ch_p3.png"))?,
                l(include_bytes!("../assets/ch_p4.png"))?,
                l(include_bytes!("../assets/ch_p5.png"))?,
                l(include_bytes!("../assets/ch_p6.png"))?,
                l(include_bytes!("../assets/ch_s1.png"))?,
                l(include_bytes!("../assets/ch_s2.png"))?,
                l(include_bytes!("../assets/ch_s3.png"))?,
                l(include_bytes!("../assets/ch_s4.png"))?,
                l(include_bytes!("../assets/ch_s5.png"))?,
                l(include_bytes!("../assets/ch_s6.png"))?,
                l(include_bytes!("../assets/wr_2.png"))?,
                l(include_bytes!("../assets/ic_3.png"))?,
            ],
        })
    }
}

