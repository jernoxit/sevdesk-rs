//! Contact writes and the measurements of the contact filters.

use super::docs::*;
use reqwest::Method;
use sevdesk::*;

/// Counts the entries of a raw list answer and how many of them satisfy `keep`.
fn count_where(text: &str, keep: impl Fn(&serde_json::Value) -> bool) -> (usize, usize) {
    let list = raw_objects(text);
    let all = list.as_array().cloned().unwrap_or_default();
    (all.len(), all.iter().filter(|o| keep(o)).count())
}

#[tokio::test]
#[ignore = "writes to the sevDesk test account"]
async fn live_write_20_contacts_create_filter_update() {
    run_docs("contacts", |ctx| async move {
        let b2b = b2b_customer(&ctx, true).await;
        let id = b2b.contact.id;
        let number = ctx.marker.clone();
        eprintln!("CREATED contact {id} customer number {number}");

        // A B2C person: no `name`, no buyer reference.
        let b2c = b2c_customer(&ctx).await;
        eprintln!(
            "CREATED contact {} (B2C) customer number {:?}",
            b2c.contact.id, b2c.contact.customer_number
        );
        assert!(b2c.contact.name.is_none());
        assert_eq!(b2c.contact.surename.as_deref(), Some("SDRS-LIVE"));
        assert!(b2c.contact.buyer_reference.is_none());

        // customerNumber filter: exact, substring, or ignored? The raw counts say.
        let exact = ok(ctx
            .raw(
                Method::GET,
                &format!("/Contact?customerNumber={number}"),
                None,
                Some("contact_list_by_customer_number"),
            )
            .await);
        let (all, hits) = count_where(&exact, |c| c["customerNumber"] == number.as_str());
        measured(&format!(
            "customerNumber=<full number>: raw {all}, with exactly that number {hits}"
        ));
        let prefix = &number[..number.len() - 4];
        let (all, hits) = count_where(
            &ok(ctx
                .raw(
                    Method::GET,
                    &format!("/Contact?customerNumber={prefix}"),
                    None,
                    None,
                )
                .await),
            |c| c["customerNumber"] == number.as_str(),
        );
        measured(&format!(
            "customerNumber=<prefix of it>: raw {all}, of which ours {hits}"
        ));
        let (all, _) = count_where(
            &ok(ctx
                .raw(
                    Method::GET,
                    "/Contact?customerNumber=SDRS-NO-SUCH-NUMBER",
                    None,
                    None,
                )
                .await),
            |_| false,
        );
        measured(&format!("customerNumber=<no such number>: raw {all}"));
        let found = ctx
            .c
            .find_contact_by_customer_number(&number)
            .await
            .unwrap();
        assert_eq!(found.unwrap().id, id);
        assert!(
            ctx.c
                .find_contact_by_customer_number("SDRS-NO-SUCH-NUMBER")
                .await
                .unwrap()
                .is_none()
        );

        // contact[id] on the two sub-resources: filtered by the server?
        let belongs = |o: &serde_json::Value| o["contact"]["id"] == id.to_string().as_str();
        let q = format!("contact[id]={id}&contact[objectName]=Contact");
        let (all, hits) = count_where(
            &ok(ctx
                .raw(
                    Method::GET,
                    &format!("/ContactAddress?{q}"),
                    None,
                    Some("contact_address_list"),
                )
                .await),
            belongs,
        );
        measured(&format!(
            "ContactAddress?contact[id]: raw {all}, of this contact {hits}"
        ));
        let (all, hits) = count_where(
            &ok(ctx
                .raw(
                    Method::GET,
                    &format!("/CommunicationWay?{q}"),
                    None,
                    Some("communication_way_list"),
                )
                .await),
            belongs,
        );
        measured(&format!(
            "CommunicationWay?contact[id]: raw {all}, of this contact {hits}"
        ));
        let addresses = ctx.c.contact_addresses(id).await.unwrap();
        let ways = ctx.c.communication_ways(id).await.unwrap();
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].street.as_deref(), Some(STREET));
        assert_eq!(ways.len(), 1);
        assert_eq!(ways[0].kind, Some(CommunicationWayType::Email));

        // The updates.
        let address = ctx
            .c
            .update_contact_address(
                addresses[0].id,
                &ContactAddressUpdate {
                    street: Some("Neue Strasse 2".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(address.street.as_deref(), Some("Neue Strasse 2"));
        measured(&format!(
            "PUT ContactAddress partial: zip kept {:?}, city kept {:?}",
            address.zip, address.city
        ));
        let way = ctx
            .c
            .update_communication_way(
                ways[0].id,
                &CommunicationWayUpdate {
                    value: Some(format!("changed-{}@example.com", ctx.marker.to_lowercase())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(way.value.unwrap().starts_with("changed-"));
        measured(&format!(
            "PUT CommunicationWay partial: main kept {:?}",
            way.main
        ));
        let contact = ctx
            .c
            .update_contact(
                id,
                &ContactUpdate {
                    name: Some(format!("{} (renamed)", b2b.name)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(contact.name.unwrap().ends_with("(renamed)"));
        assert_eq!(contact.customer_number.as_deref(), Some(number.as_str()));
        assert_eq!(contact.buyer_reference.as_deref(), Some("-"));

        // Organisation <-> person: what does `name` on a person do?
        let changed = ctx
            .c
            .update_contact(
                b2c.contact.id,
                &ContactUpdate {
                    name: Some("SDRS-LIVE Now An Organisation".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        measured(&format!(
            "PUT Contact name on a person: name {:?}, surename {:?}, familyname {:?}",
            changed.name, changed.surename, changed.familyname
        ));
    })
    .await;
}
