use sova::doc_schema;

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Create {
        pub title: String => vld::string().min(1),
        pub body: String => vld::string(),
    }
}

vld::schema! {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Update {
        pub title: String => vld::string().min(1),
        pub body: String => vld::string(),
    }
}

vld::schema! {
    #[derive(Debug, Clone)]
    pub struct IdParams {
        pub id: String => vld::string().min(1),
    }
}

doc_schema!(Create, Update, IdParams);
