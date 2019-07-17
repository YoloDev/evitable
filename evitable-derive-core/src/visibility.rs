use evitable_syn_meta_ext::PathExt;
use proc_macro2::{Ident, Span};
use syn::{parse_str, PathArguments, PathSegment, Token, VisRestricted, Visibility};

fn create_super_vis(depth: usize) -> Visibility {
  let mut code = String::new();
  code.push_str("pub(in ");
  for i in 0..depth {
    if i > 0 {
      code.push_str("::");
    }

    code.push_str("super");
  }

  code.push_str(")");
  parse_str(&code).unwrap()
}

fn modify_vis(r: &VisRestricted, depth: usize) -> Visibility {
  let mut r = r.clone();
  let ident = r.path.single_ident();
  if r.in_token.is_none() && ident.is_some() {
    // path is either self, super or crate
    let ident = ident.unwrap();
    match ident.to_string().as_ref() {
      "self" => create_super_vis(depth),
      "super" => create_super_vis(depth + 1),
      "crate" => Visibility::Restricted(r),
      _ => unreachable!(),
    }
  } else {
    r.in_token = Some(Token![in](Span::call_site()));
    let ident = &r.path.segments[0].ident;
    match ident.to_string().as_ref() {
      "self" => {
        r.path.segments[0].ident = Ident::new("super", Span::call_site());
      }
      "super" => {
        let segment = PathSegment {
          ident: Ident::new("super", Span::call_site()),
          arguments: PathArguments::None,
        };
        r.path.segments.insert(0, segment);
      }
      _ => {
        /* any other path is root based, and can be kept as is */
        ()
      }
    }
    Visibility::Restricted(r)
  }
}

pub fn inherited(vis: &Visibility, depth: usize) -> Visibility {
  match vis {
    Visibility::Public(_) => vis.clone(),
    Visibility::Crate(_) => vis.clone(),
    Visibility::Inherited => create_super_vis(depth),
    Visibility::Restricted(r) => modify_vis(r, depth),
  }
}
