use crate::{html::{Element, Html}, html_pass::system::{Aggregator, State}};

pub fn bake_template_content(
    template: State<Html>,
    content: State<Html>,
    is_implicit: bool
) -> State<Html> {
    process_template(template, content, is_implicit)
}

fn process_template(template: State<Html>, content: State<Html>, is_implicit: bool) -> State<Html> {
    let mut is_baked: bool = false;
    let State { aggregator, value } = template;
    let result = value.bake_template_content(&aggregator, &content, &mut is_baked);
    if !is_baked && is_implicit {
        return content.map_with(|x, ctx: &mut Aggregator| {
            // ctx.include(context);
            ctx.include(result.aggregator);
            x
        })
    }
    result.map_with(|x, ctx| {
        ctx.include(aggregator);
        ctx.include(content.aggregator);
        x
    })
}

impl Html {
    fn bake_template_content(self, aggregator: &Aggregator, content: &State<Html>, is_baked: &mut bool) -> State<Html> {
        match self {
            Self::Element(element) => element.bake_template_content(aggregator, content, is_baked),
            Self::Fragment(nodes) => {
                let nodes_len = nodes.len();
                let nodes = nodes
                    .into_iter()
                    .map(|x| x.bake_template_content(aggregator, content, is_baked));
                return State::flatten(nodes, Some(nodes_len)).map(Html::Fragment)
            }
            _ => State::wrap(self)
        }
    }
}

impl Element {
    fn bake_template_content(self, aggregator: &Aggregator, content: &State<Html>, is_baked: &mut bool) -> State<Html> {
        if self.tag.as_str() == "content" {
            *is_baked = true;
            return merge(content.clone(), &aggregator)
        }
        let children_len = self.children.len();
        let children = self.children
            .into_iter()
            .map(|x| x.bake_template_content(aggregator, content, is_baked))
            .collect::<Vec<_>>();
        return State::flatten(children, Some(children_len)).map(|xs| {
            Html::Element(Element { tag: self.tag, attrs: self.attrs, children: xs })
        })
    }
}

fn merge(mut content: State<Html>, aggregator: &Aggregator) -> State<Html> {
    content.aggregator.include(aggregator.clone());
    content
}
