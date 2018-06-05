use std::cell::RefCell;

use cairo::{self, ImageSurface};
use libc::{self, c_char};

use attributes::Attribute;
use handle::RsvgHandle;
use node::{boxed_node_new, NodeResult, NodeTrait, NodeType, RsvgCNodeImpl, RsvgNode};
use parsers::parse;
use property_bag::PropertyBag;
use srgb::{linearize_surface, unlinearize_surface};
use util::utf8_cstr_opt;

use super::context::{FilterContext, FilterOutput, FilterResult, IRect};
use super::input::Input;
use super::{get_surface, Filter, FilterError, Primitive};

/// The `feMerge` filter primitive.
struct Merge {
    base: Primitive,
}

/// The `<feMergeNode>` element.
struct MergeNode {
    in_: RefCell<Option<Input>>,
}

impl Merge {
    /// Constructs a new `Merge` with empty properties.
    #[inline]
    fn new() -> Merge {
        Merge {
            base: Primitive::new::<Self>(),
        }
    }
}

impl MergeNode {
    /// Constructs a new `MergeNode` with empty properties.
    #[inline]
    fn new() -> MergeNode {
        MergeNode {
            in_: RefCell::new(None),
        }
    }
}

impl NodeTrait for Merge {
    #[inline]
    fn set_atts(
        &self,
        node: &RsvgNode,
        handle: *const RsvgHandle,
        pbag: &PropertyBag,
    ) -> NodeResult {
        self.base.set_atts(node, handle, pbag)
    }

    #[inline]
    fn get_c_impl(&self) -> *const RsvgCNodeImpl {
        self.base.get_c_impl()
    }
}

impl NodeTrait for MergeNode {
    #[inline]
    fn set_atts(
        &self,
        _node: &RsvgNode,
        _handle: *const RsvgHandle,
        pbag: &PropertyBag,
    ) -> NodeResult {
        for (_key, attr, value) in pbag.iter() {
            match attr {
                Attribute::In => {
                    self.in_.replace(Some(parse("in", value, (), None)?));
                }
                _ => (),
            }
        }

        Ok(())
    }
}

impl MergeNode {
    fn render(
        &self,
        ctx: &FilterContext,
        bounds: IRect,
        output_surface: Option<ImageSurface>,
    ) -> Result<ImageSurface, FilterError> {
        let input_surface = get_surface(ctx.get_input(self.in_.borrow().as_ref()))?;
        let input_surface =
            linearize_surface(&input_surface, bounds).map_err(FilterError::BadInputSurfaceStatus)?;

        if output_surface.is_none() {
            return Ok(input_surface);
        }
        let output_surface = output_surface.unwrap();

        let cr = cairo::Context::new(&output_surface);
        cr.rectangle(
            bounds.x0 as f64,
            bounds.y0 as f64,
            (bounds.x1 - bounds.x0) as f64,
            (bounds.y1 - bounds.y0) as f64,
        );
        cr.clip();
        cr.set_source_surface(&input_surface, 0f64, 0f64);
        cr.set_operator(cairo::Operator::Over);
        cr.paint();

        Ok(output_surface)
    }
}

impl Filter for Merge {
    fn render(&self, node: &RsvgNode, ctx: &FilterContext) -> Result<FilterResult, FilterError> {
        let bounds = self.base.get_bounds(ctx);

        let mut output_surface = None;
        for child in node
            .children()
            .filter(|c| c.get_type() == NodeType::FilterPrimitiveMergeNode)
        {
            output_surface =
                Some(child.with_impl(move |c: &MergeNode| c.render(ctx, bounds, output_surface))?);
        }

        let output_surface = output_surface
            .map(|surface| unlinearize_surface(&surface, bounds))
            .unwrap_or_else(|| {
                ImageSurface::create(
                    cairo::Format::ARgb32,
                    ctx.source_graphic().get_width(),
                    ctx.source_graphic().get_height(),
                )
            })
            .map_err(FilterError::OutputSurfaceCreation)?;

        Ok(FilterResult {
            name: self.base.result.borrow().clone(),
            output: FilterOutput {
                surface: output_surface,
                bounds,
            },
        })
    }
}

/// Returns a new `feMerge` node.
#[no_mangle]
pub unsafe extern "C" fn rsvg_new_filter_primitive_merge(
    _element_name: *const c_char,
    parent: *mut RsvgNode,
    id: *const libc::c_char,
) -> *mut RsvgNode {
    let filter = Merge::new();
    boxed_node_new(
        NodeType::FilterPrimitiveMerge,
        parent,
        utf8_cstr_opt(id),
        Box::new(filter),
    )
}

/// Returns a new `feMergeNode` node.
#[no_mangle]
pub unsafe extern "C" fn rsvg_new_filter_primitive_merge_node(
    _element_name: *const c_char,
    parent: *mut RsvgNode,
    id: *const libc::c_char,
) -> *mut RsvgNode {
    let filter = MergeNode::new();
    boxed_node_new(
        NodeType::FilterPrimitiveMergeNode,
        parent,
        utf8_cstr_opt(id),
        Box::new(filter),
    )
}
