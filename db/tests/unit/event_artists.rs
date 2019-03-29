use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;

#[test]
fn create() {
    let project = TestProject::new();
    let artist = project.create_artist().finish();
    let event = project.create_event().finish();
    let rank = 1;
    let importance = 0;

    let event_artist = EventArtist::create(event.id, artist.id, rank, None, importance, None)
        .commit(None, project.get_connection())
        .unwrap();

    assert_eq!(
        event_artist.event_id, event.id,
        "Event foreign key does not match"
    );
    assert_eq!(
        event_artist.artist_id, artist.id,
        "Artist foreign key does not match"
    );
    assert_eq!(event_artist.rank, rank, "Artist rank is not correct");
    assert_eq!(
        event_artist.importance, importance,
        "Artist importance is not correct"
    );
    assert_eq!(event_artist.id.to_string().is_empty(), false);
}

#[test]
fn find_all_by_event() {
    let project = TestProject::new();
    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();
    let event = project.create_event().finish();

    let event_artist1 = EventArtist::create(event.id, artist1.id, 1, None, 0, None)
        .commit(None, project.get_connection())
        .unwrap();
    let event_artist2 = EventArtist::create(event.id, artist2.id, 2, None, 1, None)
        .commit(None, project.get_connection())
        .unwrap();

    let result = EventArtist::find_all_from_event(event.id, project.get_connection()).unwrap();

    assert_equiv!(
        vec![
            DisplayEventArtist {
                artist: artist1,
                set_time: event_artist1.set_time,
                event_id: event_artist1.event_id,
                rank: event_artist1.rank,
                importance: event_artist1.importance,
                stage_id: event_artist1.stage_id,
            },
            DisplayEventArtist {
                artist: artist2,
                set_time: event_artist2.set_time,
                event_id: event_artist2.event_id,
                rank: event_artist2.rank,
                importance: event_artist2.importance,
                stage_id: event_artist2.stage_id,
            }
        ],
        result
    );
}
