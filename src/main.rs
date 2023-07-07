use anyhow::{bail, Ok, Result};
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    http::HeaderValue,
    server::conn::http1,
    service::service_fn,
    Method, Request, Response,
};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use tokio::net::TcpListener;

macro_rules! respond_text {
    ($v:expr) => {
        Full::new(Bytes::from($v.trim().to_string()))
    };
}

const APP_NAME: &str = "Tokopedia Client API";

static HTTP_CLIENT: once_cell::sync::Lazy<reqwest::Client> =
    Lazy::new(|| reqwest::Client::builder().build().unwrap());

macro_rules! build_id {
    () => {
        build_id::get().to_string()
    };
}

macro_rules! app_desc {
    () => {
        format!("{} (build {})", APP_NAME, build_id!())
    };
}

macro_rules! load_template {
    ($n:expr) => {
        include_str!(concat!("../templates/", $n)).to_string()
    };
    ($n:expr,[$($v:expr),+]) => {{
        include_str!(concat!("../templates/", $n))$(.replace($v.0, $v.1))+
    }};
}

trait QuickParser {
    fn get_value_between<'a>(&'a self, val1: &str, val2: &str) -> Result<&'a str>;
}

impl QuickParser for String {
    fn get_value_between<'a>(&'a self, val1: &str, val2: &str) -> Result<&'a str> {
        let result = self
            .split_once(val1)
            .unwrap_or(("", ""))
            .1
            .split_once(val2)
            .unwrap_or(("", ""))
            .0;

        if result.len() <= 0 {
            bail!("");
        }

        Ok(result)
    }
}

impl QuickParser for &'static str {
    fn get_value_between<'a>(&'a self, val1: &str, val2: &str) -> Result<&'a str> {
        let result = self
            .split_once(val1)
            .unwrap_or(("", ""))
            .1
            .split_once(val2)
            .unwrap_or(("", ""))
            .0;

        if result.len() <= 0 {
            bail!("");
        }

        Ok(result)
    }
}

trait Accept {
    fn to_vec(&self) -> Result<Vec<String>>;
    fn has(&self, value: &str) -> Result<bool>;
    fn priority(&self, value: &[&str]) -> Result<String>;
}

impl Accept for HeaderValue {
    fn to_vec(&self) -> Result<Vec<String>> {
        Ok(self
            .to_str()?
            .split(",")
            .map(|v| v.trim().to_string())
            .map(|v| {
                v.split(";")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string()
            })
            .map(|v| {
                v.split("+")
                    .collect::<Vec<&str>>()
                    .get(0)
                    .unwrap()
                    .to_string()
            })
            .collect())
    }

    fn has(&self, value: &str) -> Result<bool> {
        Ok(self.to_vec()?.contains(&value.to_string()))
    }

    fn priority(&self, value: &[&str]) -> Result<String> {
        let value = self
            .to_vec()?
            .iter()
            .filter(|v| value.to_vec().contains(&v.as_str()))
            .cloned()
            .collect::<Vec<String>>();
        Ok(value.get(0).unwrap_or(&"".to_string()).to_string())
    }
}

async fn service(req: Request<Incoming>) -> Result<Response<Full<Bytes>>> {
    if req.method() == Method::HEAD {
        return Ok(Response::new(respond_text!("")));
    }

    let accept = req.headers().get("Accept");

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            if let Some(accept) = accept {
                let accept_type = accept.priority(&["text/html", "application/json"])?;

                if accept_type == "text/html" {
                    return Ok(Response::builder()
                        .header("Content-Type", "text/html")
                        .body(respond_text!(load_template!(
                            "version.html",
                            [("$title", APP_NAME), ("$build", &build_id!())]
                        )))?);
                }

                if accept_type == "application/json" {
                    return Ok(Response::builder()
                        .header("Content-Type", "application/json")
                        .body(respond_text!(json!({
                            "name": APP_NAME,
                            "build": build_id!(),
                            "success": true
                        })
                        .to_string()))?);
                }
            }

            return Ok(Response::new(respond_text!(app_desc!())));
        }
        _ => {}
    }

    let splitted_path = req
        .uri()
        .path()
        .split("/")
        .filter(|v| v.len() > 0)
        .collect::<Vec<&str>>();

    if splitted_path.len() >= 1 {
        let request_type = splitted_path[0];

        if splitted_path.len() >= 2 {
            match (req.method(), splitted_path.len(), request_type) {
                (&Method::GET, 2, "search") => {
                    let search_query = splitted_path[1];

                    let body = serde_json::json!([
                      {
                        "operationName": "SearchProductQueryV4",
                        "variables": {
                          "params": format!("device=desktop&navsource=home&ob=23&page=1&q={search_query}&related=true&rows=20&safe_search=false&scheme=https&shipping=&source=universe&st=product&start=0&topads_bucket=true")
                        },
                        "query": "query SearchProductQueryV4($params: String!) {\n  ace_search_product_v4(params: $params) {\n    header {\n      totalData\n      totalDataText\n      processTime\n      responseCode\n      errorMessage\n      additionalParams\n      keywordProcess\n      componentId\n      __typename\n    }\n    data {\n      banner {\n        position\n        text\n        imageUrl\n        url\n        componentId\n        trackingOption\n        __typename\n      }\n      backendFilters\n      isQuerySafe\n      ticker {\n        text\n        query\n        typeId\n        componentId\n        trackingOption\n        __typename\n      }\n      redirection {\n        redirectUrl\n        departmentId\n        __typename\n      }\n      related {\n        position\n        trackingOption\n        relatedKeyword\n        otherRelated {\n          keyword\n          url\n          product {\n            id\n            name\n            price\n            imageUrl\n            rating\n            countReview\n            url\n            priceStr\n            wishlist\n            shop {\n              city\n              isOfficial\n              isPowerBadge\n              __typename\n            }\n            ads {\n              adsId: id\n              productClickUrl\n              productWishlistUrl\n              shopClickUrl\n              productViewUrl\n              __typename\n            }\n            badges {\n              title\n              imageUrl\n              show\n              __typename\n            }\n            ratingAverage\n            labelGroups {\n              position\n              type\n              title\n              url\n              __typename\n            }\n            componentId\n            __typename\n          }\n          componentId\n          __typename\n        }\n        __typename\n      }\n      suggestion {\n        currentKeyword\n        suggestion\n        suggestionCount\n        instead\n        insteadCount\n        query\n        text\n        componentId\n        trackingOption\n        __typename\n      }\n      products {\n        id\n        name\n        ads {\n          adsId: id\n          productClickUrl\n          productWishlistUrl\n          productViewUrl\n          __typename\n        }\n        badges {\n          title\n          imageUrl\n          show\n          __typename\n        }\n        category: departmentId\n        categoryBreadcrumb\n        categoryId\n        categoryName\n        countReview\n        customVideoURL\n        discountPercentage\n        gaKey\n        imageUrl\n        labelGroups {\n          position\n          title\n          type\n          url\n          __typename\n        }\n        originalPrice\n        price\n        priceRange\n        rating\n        ratingAverage\n        shop {\n          shopId: id\n          name\n          url\n          city\n          isOfficial\n          isPowerBadge\n          __typename\n        }\n        url\n        wishlist\n        sourceEngine: source_engine\n        __typename\n      }\n      violation {\n        headerText\n        descriptionText\n        imageURL\n        ctaURL\n        ctaApplink\n        buttonText\n        buttonType\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n}\n"
                      }
                    ]);

                    let response = HTTP_CLIENT
                        .post("https://gql.tokopedia.com/graphql/PDPGetLayoutQuery")
                        .header("Content-Type", "application/json")
                        .header("User-Agent", "PostmanRuntime/7.32.3")
                        .body(body.to_string())
                        .send()
                        .await?
                        .text()
                        .await?;

                    let response: Value = serde_json::from_str(&response)?;

                    let data = response[0]["data"]["ace_search_product_v4"]["data"].clone();

                    let current_keyword = data["suggestion"]["currentKeyword"].as_str().unwrap();
                    let suggestion = data["suggestion"]["suggestion"].as_str().unwrap();

                    let products = data["products"].as_array().unwrap();

                    let mut result_products = Vec::new();

                    for product in products {
                        let shop_name = product["shop"]["name"].as_str().unwrap();
                        let shop_url = product["shop"]["url"].as_str().unwrap();
                        let shop_city = product["shop"]["city"].as_str().unwrap();
                        let shop_is_official = product["shop"]["isOfficial"].as_bool().unwrap();
                        let shop_is_powerbadge = product["shop"]["isPowerBadge"].as_bool().unwrap();
                        let shop_username = shop_url.replace("https://www.tokopedia.com/", "");

                        let product_name = product["name"].as_str().unwrap();
                        let product_url = product["url"].as_str().unwrap();
                        let product_price = product["price"].as_str().unwrap();
                        let product_thumbnail = product["imageUrl"].as_str().unwrap();
                        let product_category = product["categoryName"].as_str().unwrap();
                        let product_id = product_url
                            .to_string()
                            .get_value_between(&format!("{shop_username}/"), "?")?
                            .to_string();

                        result_products.push(json!({
                            "seller": {
                                "name": shop_name,
                                "id": shop_username,
                                "url": shop_url,
                                "city": shop_city,
                                "isOfficial": shop_is_official,
                                "hasPowerBadge": shop_is_powerbadge
                            },
                            "name": product_name,
                            "url": product_url,
                            "price": product_price,
                            "thumbnail": product_thumbnail,
                            "category": product_category,
                            "id": product_id
                        }));
                    }

                    return Ok(Response::builder()
                        .header("Content-Type", "application/json")
                        .body(respond_text!(json!({
                            "success": true,
                            "keyword": current_keyword,
                            "suggestion": suggestion,
                            "results": result_products
                        })
                        .to_string()))?);
                }
                (&Method::GET, 3, "lookup") => {
                    let seller = splitted_path[1];
                    let product = splitted_path[2];

                    let body = serde_json::json!([
                      {
                        "operationName": "PDPGetLayoutQuery",
                        "variables": {
                          "shopDomain": seller,
                          "productKey": product,
                          "layoutID": "",
                          "apiVersion": 1
                        },
                        "query": "fragment ProductVariant on pdpDataProductVariant {\n  errorCode\n  parentID\n  defaultChild\n  sizeChart\n  totalStockFmt\n  variants {\n    productVariantID\n    variantID\n    name\n    identifier\n    option {\n      picture {\n        urlOriginal: url\n        urlThumbnail: url100\n        __typename\n      }\n      productVariantOptionID\n      variantUnitValueID\n      value\n      hex\n      stock\n      __typename\n    }\n    __typename\n  }\n  children {\n    productID\n    price\n    priceFmt\n    optionID\n    optionName\n    productName\n    productURL\n    picture {\n      urlOriginal: url\n      urlThumbnail: url100\n      __typename\n    }\n    stock {\n      stock\n      isBuyable\n      stockWordingHTML\n      minimumOrder\n      maximumOrder\n      __typename\n    }\n    isCOD\n    isWishlist\n    campaignInfo {\n      campaignID\n      campaignType\n      campaignTypeName\n      campaignIdentifier\n      background\n      discountPercentage\n      originalPrice\n      discountPrice\n      stock\n      stockSoldPercentage\n      startDate\n      endDate\n      endDateUnix\n      appLinks\n      isAppsOnly\n      isActive\n      hideGimmick\n      isCheckImei\n      minOrder\n      __typename\n    }\n    thematicCampaign {\n      additionalInfo\n      background\n      campaignName\n      icon\n      __typename\n    }\n    __typename\n  }\n  __typename\n}\n\nfragment ProductMedia on pdpDataProductMedia {\n  media {\n    type\n    urlOriginal: URLOriginal\n    urlThumbnail: URLThumbnail\n    urlMaxRes: URLMaxRes\n    videoUrl: videoURLAndroid\n    prefix\n    suffix\n    description\n    variantOptionID\n    __typename\n  }\n  videos {\n    source\n    url\n    __typename\n  }\n  __typename\n}\n\nfragment ProductCategoryCarousel on pdpDataCategoryCarousel {\n  linkText\n  titleCarousel\n  applink\n  list {\n    categoryID\n    icon\n    title\n    isApplink\n    applink\n    __typename\n  }\n  __typename\n}\n\nfragment ProductHighlight on pdpDataProductContent {\n  name\n  price {\n    value\n    currency\n    __typename\n  }\n  campaign {\n    campaignID\n    campaignType\n    campaignTypeName\n    campaignIdentifier\n    background\n    percentageAmount\n    originalPrice\n    discountedPrice\n    originalStock\n    stock\n    stockSoldPercentage\n    threshold\n    startDate\n    endDate\n    endDateUnix\n    appLinks\n    isAppsOnly\n    isActive\n    hideGimmick\n    __typename\n  }\n  thematicCampaign {\n    additionalInfo\n    background\n    campaignName\n    icon\n    __typename\n  }\n  stock {\n    useStock\n    value\n    stockWording\n    __typename\n  }\n  variant {\n    isVariant\n    parentID\n    __typename\n  }\n  wholesale {\n    minQty\n    price {\n      value\n      currency\n      __typename\n    }\n    __typename\n  }\n  isCashback {\n    percentage\n    __typename\n  }\n  isTradeIn\n  isOS\n  isPowerMerchant\n  isWishlist\n  isCOD\n  preorder {\n    duration\n    timeUnit\n    isActive\n    preorderInDays\n    __typename\n  }\n  __typename\n}\n\nfragment ProductCustomInfo on pdpDataCustomInfo {\n  icon\n  title\n  isApplink\n  applink\n  separator\n  description\n  __typename\n}\n\nfragment ProductInfo on pdpDataProductInfo {\n  row\n  content {\n    title\n    subtitle\n    applink\n    __typename\n  }\n  __typename\n}\n\nfragment ProductDetail on pdpDataProductDetail {\n  content {\n    title\n    subtitle\n    applink\n    showAtFront\n    isAnnotation\n    __typename\n  }\n  __typename\n}\n\nfragment ProductDataInfo on pdpDataInfo {\n  icon\n  title\n  isApplink\n  applink\n  content {\n    icon\n    text\n    __typename\n  }\n  __typename\n}\n\nfragment ProductSocial on pdpDataSocialProof {\n  row\n  content {\n    icon\n    title\n    subtitle\n    applink\n    type\n    rating\n    __typename\n  }\n  __typename\n}\n\nquery PDPGetLayoutQuery($shopDomain: String, $productKey: String, $layoutID: String, $apiVersion: Float, $userLocation: pdpUserLocation, $extParam: String, $tokonow: pdpTokoNow) {\n  pdpGetLayout(shopDomain: $shopDomain, productKey: $productKey, layoutID: $layoutID, apiVersion: $apiVersion, userLocation: $userLocation, extParam: $extParam, tokonow: $tokonow) {\n    requestID\n    name\n    pdpSession\n    basicInfo {\n      alias\n      createdAt\n      isQA\n      id: productID\n      shopID\n      shopName\n      minOrder\n      maxOrder\n      weight\n      weightUnit\n      condition\n      status\n      url\n      needPrescription\n      catalogID\n      isLeasing\n      isBlacklisted\n      isTokoNow\n      menu {\n        id\n        name\n        url\n        __typename\n      }\n      category {\n        id\n        name\n        title\n        breadcrumbURL\n        isAdult\n        isKyc\n        minAge\n        detail {\n          id\n          name\n          breadcrumbURL\n          isAdult\n          __typename\n        }\n        __typename\n      }\n      txStats {\n        transactionSuccess\n        transactionReject\n        countSold\n        paymentVerified\n        itemSoldFmt\n        __typename\n      }\n      stats {\n        countView\n        countReview\n        countTalk\n        rating\n        __typename\n      }\n      __typename\n    }\n    components {\n      name\n      type\n      position\n      data {\n        ...ProductMedia\n        ...ProductHighlight\n        ...ProductInfo\n        ...ProductDetail\n        ...ProductSocial\n        ...ProductDataInfo\n        ...ProductCustomInfo\n        ...ProductVariant\n        ...ProductCategoryCarousel\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n}\n"
                      }
                    ]);

                    let response = HTTP_CLIENT
                        .post("https://gql.tokopedia.com/graphql/PDPGetLayoutQuery")
                        .header("X-Tkpd-Akamai", "pdpGetLayout")
                        .header("Content-Type", "application/json")
                        .header("User-Agent", "PostmanRuntime/7.32.3")
                        .body(body.to_string())
                        .send()
                        .await?
                        .text()
                        .await?;

                    if response.contains("product: not found") {
                        return Ok(Response::builder()
                            .header("Content-Type", "application/json")
                            .body(respond_text!(json!({
                                "reason": "Product not found",
                                "success": false
                            })
                            .to_string()))?);
                    }

                    let response: Value = serde_json::from_str(&response)?;

                    let components = response[0]["data"]["pdpGetLayout"]["components"]
                        .as_array()
                        .unwrap();
                    let basic_info = &response[0]["data"]["pdpGetLayout"]["basicInfo"];

                    let mut title = "".to_string();
                    let mut description = "".to_string();
                    let mut price = 0;
                    let mut stock = "0".to_string();

                    let store_name = basic_info["shopName"].as_str().unwrap();
                    let original_url = basic_info["url"].as_str().unwrap();
                    let created_at = basic_info["createdAt"].as_str().unwrap();

                    for component in components {
                        let component_name = component["name"].as_str().unwrap();

                        if component_name == "product_content" {
                            let data = component["data"][0].clone();

                            title = data["name"].as_str().unwrap().to_string();
                            price = data["price"]["value"].as_u64().unwrap();
                            stock = data["stock"]["value"].as_str().unwrap().to_string();
                        }

                        if component_name == "product_detail" {
                            let contents = component["data"][0]["content"].as_array().unwrap();

                            for content in contents {
                                let title = content["title"].as_str().unwrap();

                                if title == "Deskripsi" {
                                    description = content["subtitle"].as_str().unwrap().to_string();
                                }
                            }
                        }
                    }

                    return Ok(Response::builder()
                        .header("Content-Type", "application/json")
                        .body(respond_text!(json!({
                            "success": true,
                            "title": title,
                            "description": description,
                            "price": price,
                            "stock": stock.parse::<usize>()?,
                            "storeName": store_name,
                            "originalUrl": original_url,
                            "createdAt": created_at
                        })
                        .to_string()))?);
                }
                _ => {}
            }
        }
    }

    if let Some(accept) = accept {
        let accept_type = accept.priority(&["text/html", "application/json"])?;

        if accept_type == "text/html" {
            return Ok(Response::builder()
                .status(404)
                .header("Content-Type", "text/html")
                .body(respond_text!(load_template!(
                    "404.html",
                    [("$title", APP_NAME)]
                )))?);
        }

        if accept_type == "application/json" {
            return Ok(Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(respond_text!(json!({
                    "reason": "404 Not found",
                    "success": false
                })
                .to_string()))?);
        }
    }

    Ok(Response::builder()
        .status(404)
        .body(respond_text!("404 Not found"))?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:5000").await?;

    println!("{}", app_desc!());

    println!(
        "Server started at {ip_addr}:{port}",
        ip_addr = listener.local_addr()?.ip().to_string(),
        port = listener.local_addr()?.port()
    );

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service_fn(service))
                .await
            {
                eprintln!("Something is wrong: {:?}", err)
            }
        });
    }
}
