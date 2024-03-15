
class AppState {
    constructor(sensitiveCategories) {
        this.sensitiveCategories = sensitiveCategories;
        this.inJourney = false;
        this.currentUserId = undefined;
    }

    isSensitive(itemId) {
        const category = STATICS.aisleOfProduct[itemId];
        return this.sensitiveCategories.includes(category);
    }

    isSensitiveAisle(aisleId) {
        return this.sensitiveCategories.includes(aisleId);
    }
}

class App {

    constructor() {

        this.state = new AppState([27, 28, 62, 124, 134]);
        this.socket = new WebSocket("ws://localhost:8080/ws");

        this.socket.onmessage = function (event) {
            const response = JSON.parse(event.data);
            if (response["response_type"] == "purchases") {
                renderPurchases(response["payload"]["item_purchases"]);
            } else if (response["response_type"] == "model_state") {
                renderEmbedding(response["payload"]["embedding"]["weights_per_item"]);
                renderEgoNetwork(response["payload"]["ego_network"]);
                renderTopAisles(response["payload"]["ego_network"]["top_aisles"]);
            } else if (response["response_type"] == "recommendations") {
                renderRecommendations(response["payload"]);
            } else if (response["response_type"] == "deletion_impact") {
                renderUnlearningChanges(response["payload"]);
            }
        }
    }

    startJourney() {
        this.state.inJourney = True;
    }

    userFocus(userId) {

        this.state.currentUserId = userId;
        this.requestPurchases();
        this.requestModelState();
        this.requestRecommendations();
    }

    requestPurchases() {
        this.socket.send(JSON.stringify({ "Purchases": { "user_id": this.state.currentUserId } }));
    }

    requestModelState() {
        this.socket.send(JSON.stringify({ "ModelState": { "user_id": this.state.currentUserId } }));
    }

    requestRecommendations() {
        this.socket.send(JSON.stringify({ "Recommendations": { "user_id": this.state.currentUserId } }));
    }

    unlearnPurchase(itemId) {
        this.socket.send(JSON.stringify({ "PurchaseDeletion": { "user_id": this.state.currentUserId, "item_id": itemId } }));
    }
}
