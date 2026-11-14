use std::collections::HashMap;

use anyhow::{Result, anyhow};
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::vector::{Vector2F, Vector2I, vec2i};

use super::*;
use crate::platform::{self, TextLayoutSystem};

#[test]
fn test_subpixel_alignment_computation() {
    {
        // Default case - a non-fractional offset should have an alignment
        // value of zero.
        let pos = vec2f(0., 0.);
        let alignment = SubpixelAlignment::new(pos);
        assert_eq!(alignment.0, 0);
    }
    {
        // 0.1 rounds down to 0 (not up to 0.33 -> 1)
        let pos = vec2f(0.1, 0.);
        let alignment = SubpixelAlignment::new(pos);
        assert_eq!(alignment.0, 0);
    }
    {
        // y-position doesn't affect computation
        let pos = vec2f(0.1, 0.33);
        let alignment = SubpixelAlignment::new(pos);
        assert_eq!(alignment.0, 0);
    }
    {
        // 0.2 rounds up to 0.33 -> 1
        let pos = vec2f(0.2, 0.);
        let alignment = SubpixelAlignment::new(pos);
        assert_eq!(alignment.0, 1);
    }
    {
        // 0.66 doesn't round, and converts to 2
        let pos = vec2f(0.66, 0.);
        let alignment = SubpixelAlignment::new(pos);
        assert_eq!(alignment.0, 2);
    }
    {
        // 0.9 rounds up to 1.0 -> 0
        let pos = vec2f(0.9, 0.);
        let alignment = SubpixelAlignment::new(pos);
        assert_eq!(alignment.0, 0);
    }
}

struct EmWidthFontDB {
    bounds: Option<RectI>,
    advance: Option<Vector2I>,
    glyphs: Option<HashMap<(FontId, char), GlyphId>>,
}

impl EmWidthFontDB {
    fn cache(bounds: Option<RectI>, advance: Option<Vector2I>) -> Cache {
        Cache::new(Box::new(Self {
            bounds,
            advance,
            glyphs: None,
        }))
    }
}

impl platform::FontDB for EmWidthFontDB {
    fn load_from_bytes(&mut self, _name: &str, _bytes: Vec<Vec<u8>>) -> Result<FamilyId> {
        unimplemented!()
    }

    #[cfg(not(target_family = "wasm"))]
    fn load_from_system(&mut self, _font_family: &str) -> Result<FamilyId> {
        unimplemented!()
    }

    #[cfg(not(target_family = "wasm"))]
    fn load_all_system_fonts(
        &self,
    ) -> futures::future::BoxFuture<'static, Box<dyn platform::LoadedSystemFonts>> {
        unimplemented!()
    }

    #[cfg(not(target_family = "wasm"))]
    fn process_loaded_system_fonts(
        &mut self,
        _loaded_system_fonts: Box<dyn platform::LoadedSystemFonts>,
    ) -> Vec<(Option<FamilyId>, FontInfo)> {
        unimplemented!()
    }

    fn fallback_fonts(&self, _ch: char, _font_id: FontId) -> Vec<FontId> {
        vec![FontId(2)]
    }

    fn select_font(&self, family_id: FamilyId, properties: Properties) -> FontId {
        FontId(
            family_id.0
                + if properties.weight == Weight::Bold {
                    10
                } else {
                    0
                },
        )
    }

    fn font_metrics(&self, _font_id: FontId) -> Metrics {
        Metrics {
            units_per_em: 16,
            ascent: 12,
            descent: -4,
            line_gap: 0,
        }
    }

    fn glyph_advance(&self, _font_id: FontId, _glyph_id: GlyphId) -> Result<Vector2I> {
        self.advance.ok_or_else(|| anyhow!("No advance for glyph"))
    }

    fn load_family_name_from_id(&self, _id: FamilyId) -> Option<String> {
        unimplemented!()
    }

    fn glyph_raster_bounds(
        &self,
        _font_id: FontId,
        _size: f32,
        _glyph_id: GlyphId,
        _scale: Vector2F,
        _glyph_config: &crate::rendering::GlyphConfig,
    ) -> Result<RectI> {
        unimplemented!()
    }

    fn glyph_typographic_bounds(&self, _font_id: FontId, glyph_id: GlyphId) -> Result<RectI> {
        self.bounds
            .ok_or_else(|| anyhow!("No bounding box for glyph id {glyph_id}"))
    }

    fn rasterize_glyph(
        &self,
        _font_id: FontId,
        _size: f32,
        _glyph_id: GlyphId,
        _scale: Vector2F,
        _subpixel_alignment: SubpixelAlignment,
        _glyph_config: &crate::rendering::GlyphConfig,
        _format: canvas::RasterFormat,
    ) -> Result<RasterizedGlyph> {
        unimplemented!()
    }

    fn glyph_for_char(&self, font_id: FontId, char: char) -> Option<GlyphId> {
        match &self.glyphs {
            Some(glyphs) => glyphs.get(&(font_id, char)).copied(),
            None => Some(0),
        }
    }

    fn family_id_for_name(&self, _name: &str) -> Option<FamilyId> {
        unimplemented!()
    }

    fn text_layout_system(&self) -> &dyn TextLayoutSystem {
        unimplemented!()
    }
}

#[test]
fn em_width_uses_typographic_bounds_width_when_available() {
    let cache = EmWidthFontDB::cache(
        Some(RectI::new(vec2i(0, 0), vec2i(10, 12))),
        Some(vec2i(20, 0)),
    );

    assert_eq!(cache.em_width(FamilyId(0), 16.0), 10.0);
}

#[test]
fn em_width_uses_horizontal_advance_when_typographic_bounds_are_missing() {
    let cache = EmWidthFontDB::cache(None, Some(vec2i(20, 0)));

    assert_eq!(cache.em_width(FamilyId(0), 16.0), 20.0);
}

#[test]
fn em_width_does_not_panic_when_bounds_and_advance_are_missing() {
    let cache = EmWidthFontDB::cache(None, None);

    assert_eq!(cache.em_width(FamilyId(0), 16.0), 16.0);
}

#[test]
fn em_width_falls_back_when_horizontal_advance_is_zero() {
    let cache = EmWidthFontDB::cache(None, Some(vec2i(0, 0)));

    assert_eq!(cache.em_width(FamilyId(0), 16.0), 16.0);
}

fn fallback_font_cache() -> Cache {
    Cache::new(Box::new(EmWidthFontDB {
        bounds: None,
        advance: None,
        glyphs: Some(HashMap::from([
            ((FontId(0), 'A'), 1),
            ((FontId(1), 'A'), 2),
            ((FontId(1), '中'), 3),
            ((FontId(2), '中'), 4),
            ((FontId(2), '🙂'), 5),
            ((FontId(11), '中'), 6),
        ])),
    }))
}

#[test]
fn preferred_fallback_preserves_primary_glyphs() {
    let cache = fallback_font_cache();
    assert_eq!(
        cache.font_family_for_char_with_fallback(
            'A',
            FamilyId(0),
            Some(FamilyId(1)),
            Properties::default()
        ),
        FamilyId(0),
    );
}

#[test]
fn preferred_fallback_overrides_cached_system_fallback_without_changing_other_text() {
    let cache = fallback_font_cache();
    assert_eq!(
        cache.glyph_for_char(FontId(0), '中', true),
        Some((4, FontId(2)))
    );
    let family = cache.font_family_for_char_with_fallback(
        '中',
        FamilyId(0),
        Some(FamilyId(1)),
        Properties::default(),
    );
    assert_eq!(family, FamilyId(1));
    assert_eq!(
        cache.glyph_for_char(cache.select_font(family, Properties::default()), '中', true),
        Some((3, FontId(1)))
    );
    assert_eq!(
        cache.glyph_for_char(FontId(0), '中', true),
        Some((4, FontId(2)))
    );
}

#[test]
fn preferred_fallback_leaves_uncovered_characters_to_automatic_fallback() {
    let cache = fallback_font_cache();
    assert_eq!(
        cache.glyph_for_char(FontId(1), '🙂', true),
        Some((5, FontId(2)))
    );
    let family = cache.font_family_for_char_with_fallback(
        '🙂',
        FamilyId(0),
        Some(FamilyId(1)),
        Properties::default(),
    );
    assert_eq!(family, FamilyId(0));
    assert_eq!(
        cache.glyph_for_char(cache.select_font(family, Properties::default()), '🙂', true),
        Some((5, FontId(2)))
    );
}

#[test]
fn preferred_fallback_can_be_switched_and_disabled() {
    let cache = fallback_font_cache();
    for (fallback, expected) in [
        (Some(FamilyId(1)), FamilyId(1)),
        (Some(FamilyId(2)), FamilyId(2)),
        (None, FamilyId(0)),
    ] {
        assert_eq!(
            cache.font_family_for_char_with_fallback(
                '中',
                FamilyId(0),
                fallback,
                Properties::default()
            ),
            expected
        );
    }
}

#[test]
fn preferred_fallback_uses_requested_font_weight() {
    let cache = fallback_font_cache();
    let properties = Properties {
        weight: Weight::Bold,
        ..Properties::default()
    };
    let family =
        cache.font_family_for_char_with_fallback('中', FamilyId(0), Some(FamilyId(1)), properties);
    assert_eq!(family, FamilyId(1));
    assert_eq!(
        cache.glyph_for_char(cache.select_font(family, properties), '中', true),
        Some((6, FontId(11)))
    );
}
