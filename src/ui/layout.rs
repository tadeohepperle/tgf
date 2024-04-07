use crate::Aabb;
use crate::Rect;
use fontdue::LineMetrics;
use glam::{dvec2, vec2, DVec2, Vec2};
use smallvec::smallvec;
use smallvec::SmallVec;

use crate::ui::{
    element::{ComputedBounds, DivComputed, Section, TextComputed},
    element_store::ElementBox,
    font::GlyphInfo,
    Align, Axis, Div, ElementWithComputed, MainAlign, SdfFont, Text, TextSection,
};

impl ElementBox {
    pub fn layout(&mut self) {
        self.layout_in_size(DVec2::MAX, DVec2::ZERO);
    }

    pub fn layout_in_size(&mut self, size: DVec2, pos_offset: DVec2) {
        self.element_mut().get_and_set_size(size);
        self.element_mut().set_position(pos_offset);
    }

    pub fn layout_relative_to_own_size(&mut self, unit_pos: DVec2, pos_offset: DVec2) {
        let own_size = self.element_mut().get_and_set_size(DVec2::MAX);
        self.element_mut()
            .set_position(-own_size * unit_pos + pos_offset);
    }
}

impl ElementWithComputed {
    pub fn get_and_set_size(&mut self, max_size: DVec2) -> DVec2 {
        match self {
            ElementWithComputed::Div((div, computed)) => div.get_and_set_size(max_size, computed),
            ElementWithComputed::Text((text, computed)) => {
                text.get_and_set_size(max_size, computed)
            }
        }
    }

    /// assumes all sizes have been calculated
    fn set_position(&mut self, pos: DVec2) {
        match self {
            ElementWithComputed::Div((div, computed)) => div.set_position(pos, computed),
            ElementWithComputed::Text((text, computed)) => text.set_position(pos, computed),
        }
    }
}

impl Div {
    pub fn get_and_set_size(&mut self, max_size: DVec2, computed: &mut DivComputed) -> DVec2 {
        let width = self.width.map(|e| e.fixed(max_size.x));
        let height = self.height.map(|e| e.fixed(max_size.y));

        let pad_x = self.padding.left + self.padding.right;
        let pad_y = self.padding.top + self.padding.bottom;

        let size = &mut computed.bounds.size;
        let content_size = &mut computed.content_size;

        match (width, height) {
            (Some(width), Some(height)) => {
                *size = dvec2(width, height);
                let max_size = *size - dvec2(pad_x, pad_y);
                *content_size = self.get_and_set_child_sizes(max_size);
            }
            (Some(width), None) => {
                let max_size = dvec2(width - pad_x, max_size.y);
                *content_size = self.get_and_set_child_sizes(max_size);
                *size = dvec2(width, content_size.y + pad_y);
            }
            (None, Some(height)) => {
                let max_size = dvec2(max_size.x, height - pad_y);
                *content_size = self.get_and_set_child_sizes(max_size);
                *size = dvec2(content_size.x + pad_x, height);
            }
            (None, None) => {
                *content_size = self.get_and_set_child_sizes(max_size);
                *size = dvec2(content_size.x + pad_x, content_size.y + pad_y);
            }
        };

        *size
    }

    /// Returns the size the children take all together.
    fn get_and_set_child_sizes(&mut self, max_size: DVec2) -> DVec2 {
        let mut all_children_size = DVec2::ZERO;
        match self.axis {
            Axis::X => {
                for child in self.children.iter_mut() {
                    let child = child.element_mut();
                    let child_size = child.get_and_set_size(max_size);
                    // children with absolute positioning should not contribute to the size of the parent.
                    if !is_absolute(child) {
                        all_children_size.x += child_size.x;
                        all_children_size.y = all_children_size.y.max(child_size.y);
                    }
                }
            }
            Axis::Y => {
                for child in self.children.iter_mut() {
                    let child = child.element_mut();
                    let child_size = child.get_and_set_size(max_size);
                    // children with absolute positioning should not contribute to the size of the parent.

                    if !is_absolute(child) {
                        all_children_size.x = all_children_size.x.max(child_size.x);
                        all_children_size.y += child_size.y;
                    }
                }
            }
        }
        all_children_size
    }

    fn set_position(&mut self, pos: DVec2, computed: &mut DivComputed) {
        // set own position:
        computed.bounds.pos = pos + self.offset;

        // set childrens positions:
        self.set_child_positions(computed)
    }

    #[inline]
    fn set_child_positions(&mut self, own_computed: &mut DivComputed) {
        match self.axis {
            Axis::X => _monomorphized_set_child_positions::<XMain>(self, own_computed),
            Axis::Y => _monomorphized_set_child_positions::<YMain>(self, own_computed),
        }

        pub trait AssembleDisassemble {
            /// returns (main_axis, cross_axis)
            fn disassemble(v: DVec2) -> (f64, f64);
            fn assemble(main: f64, cross: f64) -> DVec2;
        }

        struct XMain;
        struct YMain;

        impl AssembleDisassemble for XMain {
            #[inline(always)]
            fn disassemble(v: DVec2) -> (f64, f64) {
                // (main_axis, cross_axis)
                (v.x, v.y)
            }
            #[inline(always)]
            fn assemble(main: f64, cross: f64) -> DVec2 {
                DVec2 { x: main, y: cross }
            }
        }

        impl AssembleDisassemble for YMain {
            #[inline(always)]
            fn disassemble(v: DVec2) -> (f64, f64) {
                // (main_axis, cross_axis)
                (v.y, v.x)
            }
            #[inline(always)]
            fn assemble(main: f64, cross: f64) -> DVec2 {
                DVec2 { x: cross, y: main }
            }
        }

        /// Gets monomorphized into two functions: One for Y being the Main Axis and one for X being the Main Axis.
        #[inline(always)]
        fn _monomorphized_set_child_positions<A: AssembleDisassemble>(
            div: &mut Div,
            computed: &DivComputed,
        ) {
            let n_children = div.children.len();
            if n_children == 0 {
                return;
            }
            let pad_x = div.padding.left + div.padding.right;
            let pad_y = div.padding.top + div.padding.bottom;

            // get computed values from the previous layout step (determine size + set own pos)
            let div_size = computed.bounds.size;
            let div_pos = computed.bounds.pos;
            let content_size = computed.content_size;

            // add variables div_size -> inner_size and div_pos -> inner_pos to be the inner size of the div (div size - padding) and the
            // top left corner of the inner area instead of the top left corner of the div itself

            let inner_size = dvec2(div_size.x - pad_x, div_size.y - pad_y); // div size - padding size on all sides
            let inner_pos = div_pos + dvec2(div.padding.left, div.padding.top);

            let (main_size, cross_size) = A::disassemble(inner_size);
            let (main_content_size, _) = A::disassemble(content_size);
            let (mut main_offset, main_step) =
                main_offset_and_step(div.main_align, main_size, main_content_size, n_children);

            let calc_cross_offset = match div.cross_align {
                Align::Start => |_: f64, _: f64| -> f64 { 0.0 },
                Align::Center => |cross_parent: f64, cross_item: f64| -> f64 {
                    (cross_parent - cross_item) * 0.5
                },
                Align::End => {
                    |cross_parent: f64, cross_item: f64| -> f64 { cross_parent - cross_item }
                }
            };

            for ch in div.children.iter_mut() {
                let ch = ch.element_mut();
                let ch_size = ch.computed_bounds_mut().size; // computed in previous step

                let (ch_main_size, ch_cross_size) = A::disassemble(ch_size);
                let cross = calc_cross_offset(cross_size, ch_cross_size);

                let ch_rel_pos: DVec2;

                if let Some(unit_pos) = absolute_unit_pos(ch) {
                    // absolute positioning still considers padding of parent (inner size);
                    let inner_offset = (inner_size - ch_size) * unit_pos.as_dvec2();
                    ch_rel_pos = inner_offset;
                } else {
                    ch_rel_pos = A::assemble(main_offset, cross);
                    main_offset += ch_main_size + main_step;
                }

                ch.set_position(ch_rel_pos + inner_pos);
            }
        }

        /// The main offset is the offset on the main axis at the start of layout.
        /// After each child with relative positioning it is incremented by the childs size, plus the step value.
        ///
        /// This function computes the initial main offset and this step value for different main axis alignment modes.
        #[inline]
        fn main_offset_and_step(
            main_align: MainAlign,
            main_size: f64,
            main_content_size: f64,
            n_children: usize,
        ) -> (f64, f64) {
            let offset: f64; // initial offset on main axis for the first child
            let step: f64; //  step that gets added for each child on main axis after its own size on main axis.
            match main_align {
                MainAlign::Start => {
                    offset = 0.0;
                    step = 0.0;
                }
                MainAlign::Center => {
                    offset = (main_size - main_content_size) * 0.5;
                    step = 0.0;
                }
                MainAlign::End => {
                    offset = main_size - main_content_size;
                    step = 0.0;
                }
                MainAlign::SpaceBetween => {
                    offset = 0.0;

                    if n_children == 1 {
                        step = 0.0;
                    } else {
                        step = (main_size - main_content_size) / (n_children - 1) as f64;
                    }
                }
                MainAlign::SpaceAround => {
                    step = (main_size - main_content_size) / n_children as f64;
                    offset = step / 2.0;
                }
            };
            (offset, step)
        }
    }
}

#[inline(always)]
fn is_absolute(element: &ElementWithComputed) -> bool {
    match &element {
        ElementWithComputed::Div((d, _)) => d.absolute.is_some(),
        ElementWithComputed::Text(_) => false,
    }
}

#[inline(always)]
fn absolute_unit_pos(element: &ElementWithComputed) -> Option<Vec2> {
    match &element {
        ElementWithComputed::Div((d, _)) => d.absolute,
        ElementWithComputed::Text(_) => None,
    }
}

impl Text {
    fn get_and_set_size(&mut self, max_size: DVec2, computed: &mut TextComputed) -> DVec2 {
        *computed = layout_text(self, max_size.x as f32);
        computed.bounds.size
    }

    fn set_position(&mut self, pos: DVec2, computed: &mut TextComputed) {
        // set own position:
        computed.bounds.pos = pos + self.offset;

        // set positions of inline elements in the text:
        for element in self.element_sections_mut() {
            // computed during text layout:
            let relative_pos_in_text = element.computed_bounds_mut().pos;
            element.set_position(computed.bounds.pos + relative_pos_in_text)
        }

        for g in computed.glyphs.iter_mut() {
            g.bounds.pos.x += computed.bounds.pos.x as f32;
            g.bounds.pos.y += computed.bounds.pos.y as f32;
        }
    }
}

pub fn layout_text(text: &mut Text, max_width: f32) -> TextComputed {
    let mut text_layout = TextLayout {
        max_width,
        glyphs: vec![],
        lines: vec![],
        current_line: LineRun::new(),
        last_non_ws_glyph_advances: smallvec![],
        element_line_indices: smallvec![],
        text_section_glyphs: smallvec![],
    };
    text_layout.layout(text);
    text_layout.finalize(text)
}

#[derive(Debug)]
struct TextLayout {
    max_width: f32,
    glyphs: Vec<GlyphBoundsAndUv>,
    text_section_glyphs: SmallVec<[std::ops::Range<usize>; 2]>,
    lines: Vec<LineRun>,
    current_line: LineRun,
    /// last chars added to the layout that stick together on linebreaks, e.g. a word.
    last_non_ws_glyph_advances: SmallVec<[XOffsetAndAdance; 16]>,
    element_line_indices: SmallVec<[usize; 4]>,
}

#[derive(Debug)]
struct XOffsetAndAdance {
    offset: f32,
    advance: f32,
}

#[derive(Debug)]
pub struct GlyphBoundsAndUv {
    pub bounds: Rect,
    pub uv: Aabb,
}

#[derive(Debug)]
pub struct LineRun {
    pub baseline_y: f32,
    /// current advance where to place the next glyph if still space
    pub advance: f32,
    pub max_metrics: LineMetrics,
    pub glyph_range: std::ops::Range<usize>,
}

impl LineRun {
    fn new() -> Self {
        LineRun {
            baseline_y: 0.0,
            advance: 0.0,
            max_metrics: LineMetrics {
                ascent: 0.0,
                descent: 0.0,
                line_gap: 0.0,
                new_line_size: 0.0,
            },
            glyph_range: 0..0,
        }
    }

    fn from_metrics(metrics: LineMetrics) -> Self {
        LineRun {
            baseline_y: 0.0,
            advance: 0.0,
            max_metrics: metrics,
            glyph_range: 0..0,
        }
    }

    fn merge_metrics_take_max(&mut self, metrics: &LineMetrics) {
        self.max_metrics.ascent = self.max_metrics.ascent.max(metrics.ascent);
        self.max_metrics.descent = self.max_metrics.descent.min(metrics.descent); // min, because descent is negative
        self.max_metrics.line_gap = self.max_metrics.line_gap.max(metrics.line_gap);
        self.max_metrics.new_line_size =
            self.max_metrics.ascent - self.max_metrics.descent + self.max_metrics.line_gap;
    }
}

impl TextLayout {
    fn layout(&mut self, text: &mut Text) {
        for section in text.sections.iter_mut() {
            match section {
                Section::Text(text) => self.layout_text_section(text),
                Section::Element {
                    element,
                    sets_line_height,
                } => self.layout_element_section(element, *sets_line_height),
            }
        }
    }

    fn layout_text_section(&mut self, text: &mut TextSection) {
        let glyphs_len_before = self.glyphs.len();

        let font_size = text.font_size;
        let font: &SdfFont = &text.font;
        let line_metrics = font.line_metrics(font_size);
        self.current_line.merge_metrics_take_max(&line_metrics);

        for ch in text.string.chars() {
            let g = font.glyph_info(ch, font_size);
            let is_white_space = ch.is_whitespace();
            debug_assert_eq!(g.uv.is_some(), !is_white_space);

            // check if the glyph still fits into the current line, if not make a new line and
            // sometimes also some of the last few glyphs have to be moved to the new line, if they form a word with ch.
            if ch == '\n' {
                self.break_line(Some(line_metrics));
                continue;
            }

            let line_break = self.current_line.advance + g.metrics.advance > self.max_width;
            if line_break {
                self.break_line(Some(line_metrics));
                if is_white_space {
                    // just break, note: the whitespace here is omitted and does not add extra space.
                    // (we do not want to have extra white space at the end of a line or at the start of a line unintentionally.)
                    self.last_non_ws_glyph_advances.clear();
                } else {
                    // now move all letters that have been part of this word before onto the next line:

                    let glyphs_n = self.glyphs.len();
                    let last_n = self.last_non_ws_glyph_advances.len();

                    let last_line = self
                        .lines
                        .last_mut()
                        .expect("after linebreak, there is a line here; qed");
                    last_line.glyph_range.end -= last_n;
                    self.current_line.glyph_range.start -= last_n;
                    for (glyph, offset_and_advance) in self.glyphs[(glyphs_n - last_n)..]
                        .iter_mut()
                        .zip(self.last_non_ws_glyph_advances.iter())
                    {
                        let offset = offset_and_advance.offset;
                        let advance = offset_and_advance.advance;
                        glyph.bounds.pos.x = self.current_line.advance + offset;
                        self.current_line.advance += advance;
                    }

                    // Also note that we do not clear the current word chars here. Should we? This is now a bit buggy maybe, if any word is longer than the
                    self.add_glyph_to_current_line(&g);
                }
            } else {
                self.add_glyph_to_current_line(&g);
            }
        }
        self.text_section_glyphs
            .push(glyphs_len_before..self.glyphs.len())
    }

    // if the glyph_info provided contains the texture uv coords (means: this is not whitespace),
    // then push the glyph onto the current line, increasing the advance of the current line.
    // if glyph is whitespace, just advance the `advance` pointer of the current line, but do not push a glyph onto the vec.
    fn add_glyph_to_current_line(&mut self, g: &GlyphInfo) {
        if let Some(uv) = g.uv {
            // non-whitespace character
            let x_offset = g.metrics.xmin;
            let y_offset = -g.metrics.ymin; // minus, because our y axis points down.

            let height = g.metrics.height;

            let pos = vec2(
                self.current_line.advance + x_offset,
                -height + y_offset, //y_offset - g.metrics.height as f32,
            );
            let size = vec2(g.metrics.width, g.metrics.height);
            let primitive = GlyphBoundsAndUv {
                bounds: Rect { pos, size },
                uv,
            };
            self.glyphs.push(primitive);
            self.last_non_ws_glyph_advances.push(XOffsetAndAdance {
                offset: x_offset,
                advance: g.metrics.advance,
            });
        } else {
            // whitespace character
            self.last_non_ws_glyph_advances.clear();
        }
        self.current_line.advance += g.metrics.advance;
    }

    fn break_line(&mut self, line_metrics: Option<LineMetrics>) {
        self.current_line.glyph_range.end = self.glyphs.len();

        let mut new_line = match line_metrics {
            Some(metrics) => LineRun::from_metrics(metrics),
            None => LineRun::new(),
        };
        new_line.glyph_range.start = self.glyphs.len();

        let old_line = std::mem::replace(&mut self.current_line, new_line);
        self.lines.push(old_line);
    }

    fn layout_element_section(&mut self, element: &mut ElementBox, sets_line_height: bool) {
        // currently only y-bounded in-text elements supported. Do not use an element with unbounded size as part of some text section.
        let element = element.element_mut();
        let element_size = element.get_and_set_size(dvec2(self.max_width as f64, f64::MAX));
        // add line break if the element does not fit into this line anymore:
        let line_break = self.current_line.advance + element_size.x as f32 > self.max_width;
        if line_break {
            self.break_line(None);
        }

        // assign the x part of the element relative position already, the relative y is assined later, when we know the fine heights of each line.
        element.computed_bounds_mut().pos.x = self.current_line.advance as f64;

        self.current_line.advance += element_size.x as f32;
        let line_index = self.lines.len();
        self.element_line_indices.push(line_index);

        // changing line metrics based on the height of the element is tricky:
        // what should change? ascent of the line? ascent and descent?
        // lets make it like this for now:

        if sets_line_height {
            self.current_line.max_metrics.ascent = self
                .current_line
                .max_metrics
                .ascent
                .max(element_size.y as f32);
        }
    }

    fn finalize(self, text: &mut Text) -> TextComputed {
        let TextLayout {
            mut glyphs,
            mut lines,
            mut current_line,
            text_section_glyphs,
            element_line_indices,
            ..
        } = self;

        // set the correct end range index on the last line and add it to the other lines.
        current_line.glyph_range.end = glyphs.len();
        lines.push(current_line);

        // calculate the y of the character baseline for each line and add it to the y position of each glyphs coordinates
        let mut base_y: f32 = 0.0;
        let mut max_line_width: f32 = 0.0;

        let len = lines.len();
        for (i, line) in lines.iter_mut().enumerate() {
            base_y += line.max_metrics.ascent;
            line.baseline_y = base_y;

            max_line_width = max_line_width.max(line.advance);

            for i in line.glyph_range.clone() {
                let glyph = &mut glyphs[i];
                glyph.bounds.pos.y += base_y;
            }
            base_y += -line.max_metrics.descent + line.max_metrics.line_gap;
            if i < len - 1 {
                base_y += text.additional_line_gap;
            }
        }

        // go over all inline elements and set their position to the baseline - descent (so the total bottom of a line).
        for (i, element) in text.element_sections_mut().enumerate() {
            let line = &lines[element_line_indices[i]];
            let bottom_y = line.baseline_y - line.max_metrics.descent; // this is > baseline_y (so more down), because descent negative.
            let computed = element.computed_bounds_mut();
            computed.pos.y = bottom_y as f64 - computed.size.y;
        }

        // todo: add a mode for centered / end aligned text layout:
        //    How? Iterate over lines a second time, shift all glyphs and all elements of that line by some amount to the right, depending on the max_width of all lines.

        let size: DVec2 = dvec2(max_line_width as f64, base_y as f64);

        TextComputed {
            bounds: ComputedBounds {
                pos: DVec2::ZERO,
                size,
            },
            glyphs,
            text_section_glyphs,
        }
    }
}
