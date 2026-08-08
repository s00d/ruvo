//! JSON-LD schema helpers.

use chrono::Utc;
use ruvo_meta::{
    Article, BreadcrumbList, FAQPage, FaqAnswer, FaqEntry, LocalBusiness, Offer, Organization,
    Product, ToJsonLd, WebSite,
};

#[test]
fn article_and_product_json_ld() {
    let a = Article {
        headline: "Hello".into(),
        author: Some("Ada".into()),
        date_published: Some(Utc::now()),
        image: Some("https://ex.com/a.jpg".into()),
        description: Some("d".into()),
    };
    let v = a.json_ld();
    assert_eq!(v["@type"], "Article");
    assert_eq!(v["@context"], "https://schema.org");
    assert_eq!(v["headline"], "Hello");

    let p = Product {
        name: "Widget".into(),
        description: Some("w".into()),
        image: None,
        sku: Some("SKU-1".into()),
    };
    let v = p.json_ld();
    assert_eq!(v["@type"], "Product");
    assert_eq!(v["sku"], "SKU-1");
}

#[test]
fn offer_breadcrumb_org_website_faq_local() {
    let offer = Offer {
        price: "9.99".into(),
        price_currency: "USD".into(),
        availability: Some("https://schema.org/InStock".into()),
        url: Some("https://ex.com/p".into()),
    };
    assert_eq!(offer.json_ld()["@type"], "Offer");

    let crumbs = BreadcrumbList::from_pairs(&[("Home", "/"), ("Blog", "/blog")]);
    assert_eq!(crumbs.item_list_element.len(), 2);
    assert_eq!(crumbs.item_list_element[0].position, 1);
    let v = crumbs.json_ld();
    assert_eq!(v["@type"], "BreadcrumbList");

    let org = Organization {
        name: "Acme".into(),
        url: Some("https://acme.test".into()),
        logo: Some("https://acme.test/logo.png".into()),
    };
    assert_eq!(org.json_ld()["name"], "Acme");

    let site = WebSite {
        name: "Site".into(),
        url: "https://site.test".into(),
    };
    assert_eq!(site.json_ld()["@type"], "WebSite");

    let faq = FAQPage {
        main_entity: vec![FaqEntry {
            name: "Q?".into(),
            accepted_answer: FaqAnswer {
                text: "A.".into(),
            },
        }],
    };
    let v = faq.json_ld();
    assert_eq!(v["@type"], "FAQPage");
    assert_eq!(v["mainEntity"][0]["@type"], "Question");
    assert_eq!(v["mainEntity"][0]["acceptedAnswer"]["text"], "A.");

    let biz = LocalBusiness {
        name: "Cafe".into(),
        address: Some("1 Main".into()),
        telephone: Some("+1".into()),
        url: None,
    };
    assert_eq!(biz.json_ld()["@type"], "LocalBusiness");
}
