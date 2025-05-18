use crate::{html::{Element, Html}, process::{OutputContext, IO}};

pub fn bake_template_content(template: IO<Html>, content: IO<Html>, is_implicit: bool) -> IO<Html> {
    let mut is_baked: bool = false;
    let IO { context, value } = template;
    let result = value.bake_template_content(&context, &content, &mut is_baked);
    if !is_baked && is_implicit {
        return content.map_with(|x, ctx| {
            // ctx.include(context);
            ctx.include(result.context);
            x
        })
    }
    result.map_with(|x, ctx| {
        ctx.include(context);
        ctx.include(content.context);
        x
    })
}

impl Html {
    fn bake_template_content(self, template_ctx: &OutputContext, content: &IO<Html>, is_baked: &mut bool) -> IO<Html> {
        match self {
            Self::Element(element) => element.bake_template_content(template_ctx, content, is_baked),
            Self::Fragment(nodes) => {
                let nodes = nodes
                    .into_iter()
                    .map(|x| x.bake_template_content(template_ctx, content, is_baked))
                    .collect::<Vec<_>>();
                return IO::flatten(nodes).map(Html::Fragment)
            }
            _ => IO::wrap(self)
        }
    }
}

impl Element {
    fn bake_template_content(self, template_ctx: &OutputContext, content: &IO<Html>, is_baked: &mut bool) -> IO<Html> {
        if self.tag.as_str() == "content" {
            *is_baked = true;
            return merge(content.clone(), &template_ctx)
        }
        let children = self.children
            .into_iter()
            .map(|x| x.bake_template_content(template_ctx, content, is_baked))
            .collect::<Vec<_>>();
        return IO::flatten(children).map(|xs| {
            Html::Element(Element { tag: self.tag, attrs: self.attrs, children: xs })
        })
    }
}

fn merge(mut content: IO<Html>, template_ctx: &OutputContext) -> IO<Html> {
    content.context.include(template_ctx.clone());
    content
}
