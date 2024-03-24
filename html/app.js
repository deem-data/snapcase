
class AppState {
    constructor(scenario) {
        this.scenario = scenario;
        this.inJourney = false;
        this.currentUserId = undefined;
    }

    isSensitive(itemId) {
        const category = STATICS.aisleOfProduct[itemId];
        return STATICS.sensitiveCategories[this.scenario].includes(category);
    }

    isSensitiveAisle(aisleId) {
        return STATICS.sensitiveCategories[this.scenario].includes(aisleId);
    }
}

class App {

    constructor() {

        this.state = new AppState([]);
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


    userFocusWithScenario(userId, scenario) {
        this.state.scenario = scenario;
        this.userFocus(userId);
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

        const scenario = this.state.scenario.charAt(0).toUpperCase() + this.state.scenario.slice(1);

        this.socket.send(JSON.stringify({ "ModelState": {
            "user_id": this.state.currentUserId,
            "scenario": scenario,
        } }));
    }

    requestRecommendations() {
        this.socket.send(JSON.stringify({ "Recommendations": { "user_id": this.state.currentUserId } }));
    }

    unlearnPurchase(itemId) {
        this.socket.send(JSON.stringify({ "PurchaseDeletion": {
            "user_id": this.state.currentUserId,
            "item_id": itemId,
        } }));
    }
}
