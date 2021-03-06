/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! CSS handling for the specified value of
//! [`image`][image]s
//!
//! [image]: https://drafts.csswg.org/css-images/#image-values

use cssparser::{Parser, ToCss};
use parser::{Parse, ParserContext};
use std::f32::consts::PI;
use std::fmt;
use url::Url;
use values::computed::ComputedValueAsSpecified;
use values::specified::{Angle, CSSColor, Length, LengthOrPercentage, UrlExtraData};
use values::specified::position::{Keyword, Position};

/// Specified values for an image according to CSS-IMAGES.
/// https://drafts.csswg.org/css-images/#image-values
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub enum Image {
    Url(Url, UrlExtraData),
    Gradient(Gradient),
}

impl ToCss for Image {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        use values::LocalToCss;
        match *self {
            Image::Url(ref url, ref _extra_data) => {
                url.to_css(dest)
            }
            Image::Gradient(ref gradient) => gradient.to_css(dest)
        }
    }
}

impl Image {
    pub fn parse(context: &ParserContext, input: &mut Parser) -> Result<Image, ()> {
        if let Ok(url) = input.try(|input| input.expect_url()) {
            match UrlExtraData::make_from(context) {
                Some(extra_data) => {
                    Ok(Image::Url(context.parse_url(&url), extra_data))
                },
                None => {
                    // FIXME(heycam) should ensure we always have a principal, etc., when
                    // parsing style attributes and re-parsing due to CSS Variables.
                    println!("stylo: skipping declaration without ParserContextExtraData");
                    Err(())
                },
            }
        } else {
            Ok(Image::Gradient(try!(Gradient::parse_function(input))))
        }
    }
}

/// Specified values for a CSS gradient.
/// https://drafts.csswg.org/css-images/#gradients
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub struct Gradient {
    /// The color stops.
    pub stops: Vec<ColorStop>,
    /// True if this is a repeating gradient.
    pub repeating: bool,
    /// Gradients can be linear or radial.
    pub gradient_kind: GradientKind,
}

impl ToCss for Gradient {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        if self.repeating {
            try!(dest.write_str("repeating-"));
        }
        match self.gradient_kind {
            GradientKind::Linear(angle_or_corner) => {
                try!(dest.write_str("linear-gradient("));
                try!(angle_or_corner.to_css(dest));
            },
            GradientKind::Radial(ref shape, position) => {
                try!(dest.write_str("radial-gradient("));
                try!(shape.to_css(dest));
                try!(dest.write_str(" at "));
                try!(position.to_css(dest));
            },
        }
        for stop in &self.stops {
            try!(dest.write_str(", "));
            try!(stop.to_css(dest));
        }
        dest.write_str(")")
    }
}

impl Gradient {
    /// Parses a gradient from the given arguments.
    pub fn parse_function(input: &mut Parser) -> Result<Gradient, ()> {
        let mut repeating = false;
        let (gradient_kind, stops) = match_ignore_ascii_case! { try!(input.expect_function()),
            "linear-gradient" => {
                try!(input.parse_nested_block(|input| {
                        let kind = try!(GradientKind::parse_linear(input));
                        let stops = try!(input.parse_comma_separated(ColorStop::parse));
                        Ok((kind, stops))
                    })
                )
            },
            "repeating-linear-gradient" => {
                repeating = true;
                try!(input.parse_nested_block(|input| {
                        let kind = try!(GradientKind::parse_linear(input));
                        let stops = try!(input.parse_comma_separated(ColorStop::parse));
                        Ok((kind, stops))
                    })
                )
            },
            "radial-gradient" => {
                try!(input.parse_nested_block(|input| {
                        let kind = try!(GradientKind::parse_radial(input));
                        let stops = try!(input.parse_comma_separated(ColorStop::parse));
                        Ok((kind, stops))
                    })
                )
            },
            "repeating-radial-gradient" => {
                repeating = true;
                try!(input.parse_nested_block(|input| {
                        let kind = try!(GradientKind::parse_radial(input));
                        let stops = try!(input.parse_comma_separated(ColorStop::parse));
                        Ok((kind, stops))
                    })
                )
            },
            _ => { return Err(()); }
        };

        Ok(Gradient {
            stops: stops,
            repeating: repeating,
            gradient_kind: gradient_kind,
        })
    }
}

/// Specified values for CSS linear or radial gradients.
/// https://drafts.csswg.org/css-images/#gradients
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub enum GradientKind {
    Linear(AngleOrCorner),
    Radial(EndingShape, Position),
}

impl GradientKind {
    /// Parses a linear gradient kind from the given arguments.
    pub fn parse_linear(input: &mut Parser) -> Result<GradientKind, ()> {
        let angle_or_corner = try!(AngleOrCorner::parse(input));
        Ok(GradientKind::Linear(angle_or_corner))
    }

    /// Parses a radial gradient from the given arguments.
    pub fn parse_radial(input: &mut Parser) -> Result<GradientKind, ()> {
        let mut needs_comma = false;
        let shape = if let Ok(shape) = EndingShape::parse(input) {
            needs_comma = true;
            shape
        } else {
            EndingShape::Circle(LengthOrKeyword::Keyword(SizeKeyword::FarthestSide))
        };

        let position = if input.try(|input| input.expect_ident_matching("at")).is_ok() {
            needs_comma = true;
            try!(Position::parse(input))
        } else {
            Position {
                horiz_keyword: Some(Keyword::Center),
                horiz_position: None,
                vert_keyword: Some(Keyword::Center),
                vert_position: None,
            }
        };

        if needs_comma {
            try!(input.expect_comma());
        }

        Ok(GradientKind::Radial(shape, position))
    }
}

/// Specified values for an angle or a corner in a linear gradient.
#[derive(Clone, PartialEq, Copy, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub enum AngleOrCorner {
    Angle(Angle),
    Corner(HorizontalDirection, VerticalDirection),
}

impl ToCss for AngleOrCorner {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        match *self {
            AngleOrCorner::Angle(angle) => angle.to_css(dest),
            AngleOrCorner::Corner(horizontal, vertical) => {
                try!(dest.write_str("to "));
                try!(horizontal.to_css(dest));
                try!(dest.write_str(" "));
                try!(vertical.to_css(dest));
                Ok(())
            }
        }
    }
}

impl Parse for AngleOrCorner {
    fn parse(input: &mut Parser) -> Result<Self, ()> {
        if input.try(|input| input.expect_ident_matching("to")).is_ok() {
            let (horizontal, vertical) =
            if let Ok(value) = input.try(HorizontalDirection::parse) {
                (Some(value), input.try(VerticalDirection::parse).ok())
            } else {
                let value = try!(VerticalDirection::parse(input));
                (input.try(HorizontalDirection::parse).ok(), Some(value))
            };
            try!(input.expect_comma());
            match (horizontal, vertical) {
                (None, Some(VerticalDirection::Top)) => {
                    Ok(AngleOrCorner::Angle(Angle(0.0)))
                },
                (Some(HorizontalDirection::Right), None) => {
                    Ok(AngleOrCorner::Angle(Angle(PI * 0.5)))
                },
                (None, Some(VerticalDirection::Bottom)) => {
                    Ok(AngleOrCorner::Angle(Angle(PI)))
                },
                (Some(HorizontalDirection::Left), None) => {
                    Ok(AngleOrCorner::Angle(Angle(PI * 1.5)))
                },
                (Some(horizontal), Some(vertical)) => {
                    Ok(AngleOrCorner::Corner(horizontal, vertical))
                }
                (None, None) => unreachable!(),
            }
        } else if let Ok(angle) = input.try(Angle::parse) {
            try!(input.expect_comma());
            Ok(AngleOrCorner::Angle(angle))
        } else {
            Ok(AngleOrCorner::Angle(Angle(PI)))
        }
    }
}

impl ComputedValueAsSpecified for AngleOrCorner {}

/// Specified values for one color stop in a linear gradient.
/// https://drafts.csswg.org/css-images/#typedef-color-stop-list
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub struct ColorStop {
    /// The color of this stop.
    pub color: CSSColor,

    /// The position of this stop. If not specified, this stop is placed halfway between the
    /// point that precedes it and the point that follows it.
    pub position: Option<LengthOrPercentage>,
}

impl ToCss for ColorStop {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        try!(self.color.to_css(dest));
        if let Some(position) = self.position {
            try!(dest.write_str(" "));
            try!(position.to_css(dest));
        }
        Ok(())
    }
}

define_css_keyword_enum!(HorizontalDirection: "left" => Left, "right" => Right);
define_css_keyword_enum!(VerticalDirection: "top" => Top, "bottom" => Bottom);

impl Parse for ColorStop {
    fn parse(input: &mut Parser) -> Result<Self, ()> {
        Ok(ColorStop {
            color: try!(CSSColor::parse(input)),
            position: input.try(LengthOrPercentage::parse).ok(),
        })
    }
}

/// Determines whether the gradient's ending shape is a circle or an ellipse.
/// If <shape> is omitted, the ending shape defaults to a circle
/// if the <size> is a single <length>, and to an ellipse otherwise.
/// https://drafts.csswg.org/css-images/#valdef-radial-gradient-ending-shape
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub enum EndingShape {
    Circle(LengthOrKeyword),
    Ellipse(LengthOrPercentageOrKeyword),
}

impl Parse for EndingShape {
    fn parse(input: &mut Parser) -> Result<Self, ()> {
        // FIXME(#13664): Normally size can come before shape keywords but currently
        // parsing fails if size comes before shape keyword.
        match_ignore_ascii_case! { try!(input.expect_ident()),
            "circle" => {
                let position = input.try(LengthOrKeyword::parse).unwrap_or(
                    LengthOrKeyword::Keyword(SizeKeyword::FarthestSide));
                Ok(EndingShape::Circle(position))
            },
            "ellipse" => {
              let length = input.try(LengthOrPercentageOrKeyword::parse)
                                .unwrap_or(LengthOrPercentageOrKeyword::Keyword(SizeKeyword::FarthestSide));
                Ok(EndingShape::Ellipse(length))
            },
            _ => {
                // If two <length> is present, it defaults to ellipse, otherwise defaults to circle.
                if let Ok(length) = LengthOrPercentageOrKeyword::parse(input) {
                    if let LengthOrPercentageOrKeyword::Keyword(keyword) = length {
                        // A single keyword is valid for both ellipse and circle, but we default to circle.
                        // The grammar for ending shapes for circle and ellipse have overlap so we cannot simply
                        // try to parse as circle first
                        Ok(EndingShape::Circle(LengthOrKeyword::Keyword(keyword)))
                    } else {
                        Ok(EndingShape::Ellipse(length))
                    }
                } else {
                    // If both shape and size are omitted, we do not parse as an EndingShape
                    // Instead, GradientKind::parse_radial will go ahead and parse the stops
                    // This is necessary because GradientKind::parse_radial needs to know
                    // whether or not to expect a comma
                    Ok(EndingShape::Circle(try!(input.try(LengthOrKeyword::parse))))
                }
            }
        }
    }
}

impl ToCss for EndingShape {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        match *self {
            EndingShape::Circle(ref length) => {
                try!(dest.write_str("circle "));
                try!(length.to_css(dest));
            },
            EndingShape::Ellipse(ref length) => {
                try!(dest.write_str("ellipse "));
                try!(length.to_css(dest));
            },
        }
        Ok(())
    }
}

/// https://drafts.csswg.org/css-images/#valdef-radial-gradient-size
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub enum LengthOrKeyword {
    Length(Length),
    Keyword(SizeKeyword),
}

impl Parse for LengthOrKeyword {
    fn parse(input: &mut Parser) -> Result<Self, ()> {
        if let Ok(keyword) = input.try(SizeKeyword::parse) {
            Ok(LengthOrKeyword::Keyword(keyword))
        } else {
            Ok(LengthOrKeyword::Length(try!(Length::parse(input))))
        }
    }
}

impl ToCss for LengthOrKeyword {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        match *self {
            LengthOrKeyword::Length(ref length) => length.to_css(dest),
            LengthOrKeyword::Keyword(keyword) => keyword.to_css(dest),
        }
    }
}

/// https://drafts.csswg.org/css-images/#valdef-radial-gradient-size
#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub enum LengthOrPercentageOrKeyword {
    LengthOrPercentage(LengthOrPercentage, LengthOrPercentage),
    Keyword(SizeKeyword),
}


impl Parse for LengthOrPercentageOrKeyword {
    fn parse(input: &mut Parser) -> Result<Self, ()> {
        if let Ok(keyword) = input.try(SizeKeyword::parse) {
            Ok(LengthOrPercentageOrKeyword::Keyword(keyword))
        } else {
            Ok(LengthOrPercentageOrKeyword::LengthOrPercentage(try!(LengthOrPercentage::parse(input)),
                                                               try!(LengthOrPercentage::parse(input))))
        }
    }
}

impl ToCss for LengthOrPercentageOrKeyword {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        match *self {
            LengthOrPercentageOrKeyword::LengthOrPercentage(ref first_len, second_len) => {
                try!(first_len.to_css(dest));
                try!(dest.write_str(" "));
                second_len.to_css(dest)
            },
            LengthOrPercentageOrKeyword::Keyword(keyword) => keyword.to_css(dest),
        }
    }
}

/// https://drafts.csswg.org/css-images/#typedef-extent-keyword
define_css_keyword_enum!(SizeKeyword: "closest-side" => ClosestSide, "farthest-side" => FarthestSide,
                         "closest-corner" => ClosestCorner, "farthest-corner" => FarthestCorner);
