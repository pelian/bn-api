use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;
use uuid::Uuid;
use validator::Validate;

#[test]
fn commit() {
    let project = TestProject::new();
    let name = "Name";
    let bio = "Bio";
    let website_url = "http://www.example.com";

    let artist = Artist::create(name, None, bio, website_url)
        .commit(project.get_connection())
        .unwrap();
    assert_eq!(name, artist.name);
    assert_eq!(bio, artist.bio);
    assert_eq!(website_url, artist.website_url.unwrap());
    assert_eq!(artist.id.to_string().is_empty(), false);
}

#[test]
fn new_artist_validate() {
    let name = "Name";
    let bio = "Bio";
    let website_url = "invalid.com";

    let mut artist = Artist::create(name, None, bio, website_url);

    artist.image_url = Some("invalid".into());
    artist.thumb_image_url = Some("invalid".into());

    artist.youtube_video_urls = Some(vec!["h".into()]);

    let result = artist.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("image_url"));
    assert_eq!(errors["image_url"].len(), 1);
    assert_eq!(errors["image_url"][0].code, "url");
    assert_eq!(
        &errors["image_url"][0].message.clone().unwrap().into_owned(),
        "Image URL is invalid"
    );

    assert!(errors.contains_key("youtube_video_urls"));
    assert_eq!(errors["youtube_video_urls"].len(), 1);
    assert_eq!(errors["youtube_video_urls"][0].code, "url");
    assert_eq!(
        &errors["youtube_video_urls"][0]
            .message
            .clone()
            .unwrap()
            .into_owned(),
        "URL is invalid"
    );

    assert!(errors.contains_key("website_url"));
    assert_eq!(errors["website_url"].len(), 1);
    assert_eq!(errors["website_url"][0].code, "url");
    assert_eq!(
        &errors["website_url"][0]
            .message
            .clone()
            .unwrap()
            .into_owned(),
        "Website URL is invalid"
    );
}

#[test]
fn artist_search() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let owner = project.create_user().finish();
    let member = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&member, Roles::OrgMember)
        .finish();

    let mut artists = vec![];
    artists.push(
        project
            .create_artist()
            .with_name("The artists".to_string())
            .with_organization(&organization)
            .finish(),
    );

    let sample_artists = vec!["More artists", "Too many artists"];

    for name in &sample_artists {
        artists.push(project.create_artist().with_name(name.to_string()).finish());
    }

    artists.push(
        project
            .create_artist()
            .with_name("Hidden artists".to_string())
            .with_organization(&organization)
            .make_private()
            .finish(),
    );

    // Logged out search
    let found_artists =
        Artist::search(&None, Some("many".to_string()), connection).expect("No artists found");
    assert_eq!(1, found_artists.len());
    assert!(found_artists.iter().any(|a| a.name == sample_artists[1]));

    //  Logged out search, no filter query
    let found_artists = Artist::search(&None, None, connection).expect("No artists found");
    assert_eq!(3, found_artists.len());
    for name in &sample_artists {
        assert!(found_artists.iter().any(|a| a.name == *name));
    }

    // Owner search, with filter query
    let found_artists = Artist::search(&Some(owner), Some("artist".to_string()), connection)
        .expect("No artists found");
    assert_eq!(4, found_artists.len());
    for name in &sample_artists {
        assert!(found_artists.iter().any(|a| a.name == *name));
    }

    // Member search, no filter query
    let found_artists = Artist::search(&Some(member), None, connection).expect("No artists found");
    assert_eq!(4, found_artists.len());
    for name in &sample_artists {
        assert!(found_artists.iter().any(|a| a.name == *name));
    }
}

#[test]
fn new_artist_merge() {
    let mut artist1: NewArtist = Default::default();
    let bio = "Artist formally known as Default::default()";
    let test_url = "http://test.test".to_string();
    let artist2 = NewArtist {
        name: "Override".to_string(),
        bio: bio.to_string(),
        website_url: Some(test_url.clone()),
        image_url: Some(test_url.clone()),
        thumb_image_url: Some(test_url.clone()),
        youtube_video_urls: Some(vec![test_url.clone()]),
        facebook_username: Some("fbusername".to_string()),
        spotify_id: Some("fakespotify".to_string()),
        ..Default::default()
    };

    artist1.merge(artist2);
    assert_eq!(bio, artist1.bio);
    assert_eq!(test_url, artist1.website_url.unwrap());
    assert_eq!(test_url, artist1.image_url.unwrap());
    assert_eq!(test_url, artist1.thumb_image_url.unwrap());
    assert_eq!(test_url, artist1.youtube_video_urls.unwrap()[0]);
    assert_eq!("fbusername", artist1.facebook_username.unwrap());
    assert_eq!("fakespotify", artist1.spotify_id.unwrap());
}

#[test]
fn artist_editable_attributes_validate() {
    let mut artist_parameters = ArtistEditableAttributes::new();
    artist_parameters.name = Some("New Name".into());
    artist_parameters.bio = Some("Bio".into());
    artist_parameters.website_url = Some(Some("invalid.com".into()));
    artist_parameters.youtube_video_urls = Some(vec!["invalid".to_string()]);

    let result = artist_parameters.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("website_url"));
    assert_eq!(errors["website_url"].len(), 1);
    assert_eq!(errors["website_url"][0].code, "url");
    assert_eq!(
        &errors["website_url"][0]
            .message
            .clone()
            .unwrap()
            .into_owned(),
        "Website URL is invalid"
    );

    assert!(errors.contains_key("youtube_video_urls"));
    assert_eq!(errors["youtube_video_urls"].len(), 1);
    assert_eq!(errors["youtube_video_urls"][0].code, "url");
    assert_eq!(
        &errors["youtube_video_urls"][0]
            .message
            .clone()
            .unwrap()
            .into_owned(),
        "URL is invalid"
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let artist = project.create_artist().finish();

    let found_artist = Artist::find(&artist.id, connection).expect("Artist was not found");
    assert_eq!(found_artist.id, artist.id);
    assert_eq!(found_artist.name, artist.name);

    assert!(
        match Artist::find(&Uuid::new_v4(), connection) {
            Ok(_artist) => false,
            Err(_e) => true,
        },
        "Artist incorrectly returned when id invalid"
    );
}

#[test]
fn set_privacy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut artist = project.create_artist().finish();
    assert!(!artist.is_private);

    artist = artist.set_privacy(true, connection).unwrap();
    assert!(artist.is_private);

    artist = artist.set_privacy(false, connection).unwrap();
    assert!(!artist.is_private);
}

#[test]
fn all() {
    let project = TestProject::new();
    let conn = &project.get_connection();
    let name = "Name";
    let owner = project.create_user().finish();
    let user = project.create_user().finish();
    let admin = project
        .create_user()
        .finish()
        .add_role(Roles::Admin, conn)
        .unwrap();

    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&user, Roles::OrgMember)
        .finish();

    let artist = project.create_artist().with_name(name.into()).finish();
    assert_eq!(name, artist.name);
    assert_eq!(artist.id.to_string().is_empty(), false);

    let found_artists = Artist::all(None, conn).unwrap();
    assert_eq!(1, found_artists.len());
    assert_eq!(found_artists[0].id, artist.id);
    assert_eq!(found_artists[0].name, artist.name);

    let name2 = "Name 2";
    let artist2 = project
        .create_artist()
        .with_name(name2.into())
        .with_organization(&organization)
        .make_private()
        .finish();
    assert_eq!(name2, artist2.name);
    assert_eq!(artist2.id.to_string().is_empty(), false);

    let found_artists = Artist::all(None, conn).unwrap();
    assert_eq!(1, found_artists.len());
    assert_eq!(found_artists[0].id, artist.id);
    assert_eq!(found_artists[0].name, artist.name);

    let found_artists_owner = Artist::all(Some(&owner), conn).unwrap();
    let found_artists_user = Artist::all(Some(&user), conn).unwrap();
    let found_artists_admin = Artist::all(Some(&admin), conn).unwrap();
    assert_eq!(2, found_artists_user.len());
    assert_eq!(2, found_artists_owner.len());
    assert_eq!(2, found_artists_admin.len());
    assert_eq!(found_artists_user[0].id, artist.id);
    assert_eq!(found_artists_user[0].name, artist.name);
    assert_eq!(found_artists_user[1].id, artist2.id);
    assert_eq!(found_artists_user[1].name, artist2.name);
    assert_eq!(found_artists_admin[1].id, artist2.id);
    assert_eq!(found_artists_admin[1].name, artist2.name);
}

#[test]
fn update() {
    let project = TestProject::new();
    let name = "Old Name";
    let artist = project.create_artist().with_name(name.into()).finish();

    println!(
        "Created at: {}, updated at:{}",
        artist.created_at, artist.updated_at
    );

    let mut artist_parameters = ArtistEditableAttributes::new();
    artist_parameters.name = Some("New Name".into());
    artist_parameters.bio = Some("Bio".into());
    artist_parameters.website_url = Some(Some("http://www.example.com".into()));
    let updated_artist = artist
        .update(&artist_parameters, &project.get_connection())
        .unwrap();

    assert_eq!(updated_artist.id, artist.id);
    assert_ne!(updated_artist.name, artist.name);
    assert_eq!(updated_artist.name, artist_parameters.name.unwrap());
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let artist = project.create_artist().finish();
    assert!(artist.destroy(project.get_connection()).unwrap() > 0);
    assert!(Artist::find(&artist.id, project.get_connection()).is_err());
}

#[test]
fn organization() {
    let project = TestProject::new();
    let organization = project.create_organization().finish();
    let artist = project
        .create_artist()
        .with_organization(&organization)
        .finish();
    let artist2 = project.create_artist().finish();

    assert_eq!(
        Ok(Some(organization)),
        artist.organization(project.get_connection())
    );
    assert_eq!(Ok(None), artist2.organization(project.get_connection()));
}

#[test]
fn find_for_organization() {
    let project = TestProject::new();
    let owner = project.create_user().finish();
    let member = project.create_user().finish();
    let user = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&member, Roles::OrgMember)
        .finish();
    let artist1 = project
        .create_artist()
        .with_name("Artist1".to_string())
        .with_organization(&organization)
        .finish();

    let artist2 = project
        .create_artist()
        .with_name("Artist2".to_string())
        .with_organization(&organization)
        .finish();

    let artist3 = project
        .create_artist()
        .with_name("Artist3".to_string())
        .with_organization(&organization)
        .make_private()
        .finish();

    // Add another artist for another org to make sure it isn't included
    let organization2 = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let artist4 = project
        .create_artist()
        .with_name("Artist4".to_string())
        .with_organization(&organization2)
        .finish();

    let user = project.create_user().finish();

    let mut all_artists = vec![artist1, artist2];

    let found_artists =
        Artist::find_for_organization(None, organization.id, project.get_connection()).unwrap();
    assert_eq!(found_artists, all_artists);

    let found_artists =
        Artist::find_for_organization(None, organization.id, project.get_connection()).unwrap();
    assert_eq!(found_artists, all_artists);
    assert!(!found_artists.contains(&artist3));
    assert!(!found_artists.contains(&artist4));

    // Private artist is not shown for users
    let found_artists =
        Artist::find_for_organization(Some(user.id), organization.id, project.get_connection())
            .unwrap();
    assert_eq!(found_artists, all_artists);

    // Private artist is shown for owners and members
    all_artists.push(artist3);
    let found_artists =
        Artist::find_for_organization(Some(owner.id), organization.id, project.get_connection())
            .unwrap();
    assert_eq!(found_artists, all_artists);

    let found_artists =
        Artist::find_for_organization(Some(member.id), organization.id, project.get_connection())
            .unwrap();
    assert_eq!(found_artists, all_artists);
}
