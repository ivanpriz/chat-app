// In this struct we store what streams each user connection should listen to.
// We listen to stream, and when we get new message, it has room id in it.
// We store all users/connections belonging to this room in this struct and
// route the message to them.
pub struct ChatRoomStreamsService {
    user_id_to_rooms_ids_map: HashMap<String, Vec<String>>,
    room_id_to_user_ids_map: HashMap<String, Vec<String>>,
}
