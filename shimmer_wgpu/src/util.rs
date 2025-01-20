#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

impl Point {
    #[inline(always)]
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dimensions {
    pub width: u16,
    pub height: u16,
}

impl Dimensions {
    #[inline(always)]
    pub fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub top_left: Point,
    pub dimensions: Dimensions,
}

impl Rect {
    #[inline(always)]
    pub const fn new(top_left: Point, dimensions: Dimensions) -> Self {
        assert!(top_left.x.checked_add(dimensions.width).is_some());
        assert!(top_left.y.checked_add(dimensions.height).is_some());

        Self {
            top_left,
            dimensions,
        }
    }

    #[inline(always)]
    pub const fn from_extremes(top_left: Point, bottom_right: Point) -> Self {
        assert!(top_left.x <= bottom_right.x);
        assert!(top_left.y <= bottom_right.y);

        Self::new(top_left, Dimensions {
            width: bottom_right.x - top_left.x,
            height: bottom_right.y - top_left.y,
        })
    }

    #[inline(always)]
    pub const fn bottom_right(&self) -> Point {
        Point {
            x: self.top_left.x + self.dimensions.width,
            y: self.top_left.y + self.dimensions.height,
        }
    }

    #[inline(always)]
    pub const fn inclusive_bottom_right(&self) -> Option<Point> {
        if self.is_empty() {
            None
        } else {
            Some(Point {
                x: self.top_left.x + self.dimensions.width - 1,
                y: self.top_left.y + self.dimensions.height - 1,
            })
        }
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.dimensions.width == 0 || self.dimensions.height == 0
    }

    #[inline(always)]
    pub const fn contains(&self, point: Point) -> bool {
        (self.top_left.x <= point.x)
            && (point.x < self.top_left.x + self.dimensions.width)
            && (self.top_left.y <= point.y)
            && (point.y < self.top_left.y + self.dimensions.height)
    }

    #[inline(always)]
    pub const fn contains_rect(&self, other: &Rect) -> bool {
        let Some(other_bottom_right) = other.inclusive_bottom_right() else {
            return false;
        };

        !self.is_empty()
            && !other.is_empty()
            && self.contains(other.top_left)
            && self.contains(other_bottom_right)
    }

    #[inline(always)]
    pub const fn is_completely_below(&self, other: &Rect) -> bool {
        self.top_left.y >= other.bottom_right().y
    }

    #[inline(always)]
    pub const fn is_completely_to_the_right(&self, other: &Rect) -> bool {
        self.top_left.x >= other.bottom_right().x
    }

    #[inline(always)]
    pub const fn overlaps(&self, other: &Rect) -> bool {
        !(self.is_empty()
            || other.is_empty()
            || self.is_completely_below(other)
            || other.is_completely_below(self)
            || self.is_completely_to_the_right(other)
            || other.is_completely_to_the_right(self))
    }
}
