use super::*;

// Measured 2026-11-22 and NOT offered: `limit`/`offset` are not measured for
// contacts, so there is no `page`.

/// What `depth` selects. It WIDENS the list, it is not a narrowing filter
/// (measured 2026-11-22: `0` 150 of 150 contacts, `1` 159: 9 more).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactDepth {
    /// `depth=0`: organisations only, the default.
    OrganisationsOnly,
    /// `depth=1`: organisations AND persons, as the spec says.
    WithPersons,
}

/// `GET /Contact`.
///
/// Every filter below is measured; sevDesk silently IGNORES a filter it does
/// not understand. Date bounds are Unix seconds; text filters ignore case.
#[derive(Debug, Clone, Default)]
pub struct ContactQuery {
    /// SUBSTRING match, ignoring case: compare exactly on the result.
    pub name: Option<String>,
    /// EXACT match (measured 2026-10-01).
    pub customer_number: Option<String>,
    /// `category[id]`: exact (measured 2026-11-22).
    pub category: Option<ContactCategoryId>,
    /// `city`: PREFIX match, ignoring case, over the `ContactAddress`
    /// (measured 2026-11-22: a middle piece finds nothing).
    pub city: Option<String>,
    /// `zip`: exact, over the `ContactAddress` (measured 2026-11-22).
    pub zip: Option<String>,
    /// `country[id]`: exact, over the `ContactAddress` (measured 2026-11-22).
    pub country: Option<CountryId>,
    /// `parent[id]`: narrows to the children of that contact; only the
    /// narrowing is measured, not the completeness (2026-11-22).
    pub parent: Option<ContactId>,
    /// `create >= T`, inclusive (measured 2026-11-22).
    pub create_after: Option<OffsetDateTime>,
    /// `create <= T`, inclusive (measured 2026-11-22).
    pub create_before: Option<OffsetDateTime>,
    /// `update >= T`, inclusive (measured 2026-11-22).
    pub update_after: Option<OffsetDateTime>,
    /// `update <= T`, inclusive (measured 2026-11-22).
    pub update_before: Option<OffsetDateTime>,
    /// One tag, sent as `tags[0][id]` (measured 2026-11-22).
    pub tag: Option<TagId>,
    /// `depth`: a mode, not a filter; see [`ContactDepth`] (measured 2026-11-22).
    pub depth: Option<ContactDepth>,
    /// `orderByCustomerNumber`; `DESC` is the exact reverse of `ASC`; the key
    /// is presumably `customerNumber`, not determined (measured 2026-11-22).
    pub order_by_customer_number: Option<SortOrder>,
}

impl ContactQuery {
    pub(crate) fn pairs(&self) -> Pairs {
        let Self {
            name,
            customer_number,
            category,
            city,
            zip,
            country,
            parent,
            create_after,
            create_before,
            update_after,
            update_before,
            tag,
            depth,
            order_by_customer_number,
        } = self;
        let mut p = Pairs::new();
        push_text(&mut p, "name", name);
        push_text(&mut p, "customerNumber", customer_number);
        push_ref(&mut p, ("category[id]", "category[objectName]"), *category);
        push_text(&mut p, "city", city);
        push_text(&mut p, "zip", zip);
        push_ref(&mut p, ("country[id]", "country[objectName]"), *country);
        push_ref(&mut p, ("parent[id]", "parent[objectName]"), *parent);
        push_time(&mut p, "createAfter", *create_after);
        push_time(&mut p, "createBefore", *create_before);
        push_time(&mut p, "updateAfter", *update_after);
        push_time(&mut p, "updateBefore", *update_before);
        push_tag(&mut p, *tag);
        if let Some(v) = depth {
            let code = match v {
                ContactDepth::OrganisationsOnly => "0",
                ContactDepth::WithPersons => "1",
            };
            p.push(("depth", code.into()));
        }
        push_order(&mut p, "orderByCustomerNumber", *order_by_customer_number);
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn every_filter_is_encoded_as_measured() {
        let q = ContactQuery {
            name: Some("Acme".into()),
            customer_number: Some("K-1".into()),
            category: Some(ContactCategoryId::new(3)),
            city: Some("Ber".into()),
            zip: Some("10115".into()),
            country: Some(CountryId::new(1)),
            parent: Some(ContactId::new(4)),
            create_after: Some(datetime!(2026-11-22 12:00 UTC)),
            create_before: Some(datetime!(2026-11-22 12:00:01 UTC)),
            update_after: Some(datetime!(2026-11-22 12:00:02 UTC)),
            update_before: Some(datetime!(2026-11-22 12:00:03 UTC)),
            tag: Some(TagId::new(5)),
            depth: Some(ContactDepth::WithPersons),
            order_by_customer_number: Some(SortOrder::Desc),
        };
        let expected: Vec<(&str, String)> = vec![
            ("name", "Acme".into()),
            ("customerNumber", "K-1".into()),
            ("category[id]", "3".into()),
            ("category[objectName]", "Category".into()),
            ("city", "Ber".into()),
            ("zip", "10115".into()),
            ("country[id]", "1".into()),
            ("country[objectName]", "StaticCountry".into()),
            ("parent[id]", "4".into()),
            ("parent[objectName]", "Contact".into()),
            ("createAfter", "1795348800".into()),
            ("createBefore", "1795348801".into()),
            ("updateAfter", "1795348802".into()),
            ("updateBefore", "1795348803".into()),
            ("tags[0][id]", "5".into()),
            ("tags[0][objectName]", "Tag".into()),
            ("depth", "1".into()),
            ("orderByCustomerNumber", "DESC".into()),
        ];
        assert_eq!(q.pairs(), expected);
    }
}
