pub struct Context {}

pub struct Query;

#[juniper::graphql_object(context = Context)]
impl Query {
    async fn api(ctx: &Context) -> juniper::FieldResult<f64>  {
        Ok(0.1)
    }
}